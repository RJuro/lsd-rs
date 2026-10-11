#include "fun_head_fast.h"

/****************************************************************************

	Network operations test model
	=============================

	Exercises the network macros not used by the Nets example: loading and
	saving Pajek files, node and link operations, drawing with probabilities,
	copies and deletion of node objects, nets in several parents.

	Three regions, each with its own network of Agent nodes:
	1 - small-world, 2 - loaded from a Pajek file, 3 - circle.
	At t = 3 region 1 is copied (as region 4), whose agents are then not nodes.

 ****************************************************************************/

MODELBEGIN


EQUATION( "Setup" )
/*
Write the Pajek file loaded by region 2: comments, names, weights, vertices
without name, lower case section header, *Arcs and *Edges sections
*/

char fname[ 100 ];

sprintf( fname, "netsops_in_%d.net", seed - 1 );
f = fopen( fname, "w" );
fprintf( f, "%% input network for NetsOps\n*Vertices 8\n" );
fprintf( f, "1 \"alpha\" 0.1 0.2\n  2 \"beta\"\n3 \"gamma delta\"\n4\n" );
fprintf( f, "%% a comment between vertices\n5 \"e\"\n6 \"f\"\n7 \"g\"\n8 \"h\"\n" );
fprintf( f, "*arcs\n1 2\n1 3 2.5\n2 3 0.5\n3 1\n%% another comment\n4 5 7\n" );
fprintf( f, "*Edges\n5 6 1.5\n6 7\n7 8 3\n" );
fprintf( f, "*ARCS\n8 1 9\n1 8 0.25\n" );
fclose( f );

PARAMETER;
RESULT( 1 )


EQUATION( "Report" )
/*
Statistics of the network of every region (STAT_NETS), to follow its changes
*/

v[9] = 0;
CYCLE( cur, "Region" )
{
	v[8] = STAT_NETS( cur, "Agent" );		// NaN if the objects are not nodes
	if ( v[8] != v[8] )
		v[9] += 1e6;
	else
		v[9] += v[0] + 10 * v[1] + 100 * v[2] + 1000 * v[3] + 10000 * v[4] + 100000 * v[5];
}

if ( t == 9 )								// last step: operations on the networks from outside
{
	char nm[ 100 ];

	CYCLE( cur, "Region" )
	{
		h = (int) VS( cur, "regionId" );
		sprintf( nm, "netsops_s%d", h );
		v[9] += SAVE_NETS( cur, "Agent", nm );
		v[9] += SNAP_NETS( cur, "Agent", nm );

		cur1 = RNDDRAW_NODES( cur, "Agent" );
		curl = RNDDRAW_LINKS( cur1 );
		v[9] += curl == NULL ? 1 : V_LINK( curl );
		WRITE_NODENAMES( cur1, "last" );
		v[9] += strlen( V_NODENAMES( cur1 ) );

		if ( h == 2 )						// load the file again over the existing nodes (one is missing)
			v[9] += LOAD_NETS( cur, "Agent", "netsops_in" );

		if ( h == 3 )						// delete the network and create another on the same objects
		{
			DELETE_NETS( cur, "Agent" );
			v[8] = STAT_NETS( cur, "Agent" );
			v[9] += v[8] != v[8] ? 11 : 1;
			v[9] += INIT_NETS( cur, "Agent", "UNIFORM", 0, 3, 0 );
		}
	}
}

RESULT( v[9] )


EQUATION( "Build" )
/*
Create the network of the region
*/

v[0] = V( "regionId" );

if ( v[0] == 1 )
	v[1] = INIT_NET( "Agent", "SMALL-WORLD", 20, 4, 0.3 );
else
	if ( v[0] == 2 )
		v[1] = p->read_file_net( "Agent", "", "netsops_in", seed - 1, "net" );	// NETWORK_LOAD (deprecated macro)
	else
		v[1] = INIT_NET( "Agent", "CIRCLE", 12, 4, 0 );

STAT_NET( "Agent" );

PARAMETER;
RESULT( v[1] + 1000 * v[0] )


EQUATION( "Ops" )
/*
Operations on the network of the region, depending on time and region
*/

char nm[ 100 ];

h = (int) V( "regionId" );
v[9] = 0;

switch ( t )
{
	case 2:									// draws and searches
		for ( i = 0; i < 4; ++i )
		{
			cur = RNDDRAW_NODE( "Agent" );
			v[9] += V_NODEIDS( cur ) + STAT_NODES( cur );
		}

		cur = SEARCH_NODE( "Agent", 5 );
		v[9] += cur == NULL ? 1000 : V_NODEIDS( cur ) * 7;

		cur = SEARCH_NODE( "Agent", 9999 );	// does not exist
		v[9] += cur == NULL ? 5 : 0;
	break;

	case 3:									// new objects in a network get isolated nodes
		if ( h == 4 )						// (the copy of the region is made later in this step)
			break;

		cur = RNDDRAW_NODE( "Agent" );
		cur1 = ADDOBJ_EX( "Agent", cur );	// copy of an example
		cur2 = ADDOBJ( "Agent" );			// copy of the blueprint
		cur3 = RNDDRAW_NODE( "Agent" );
		v[9] += STAT_NODES( cur1 ) + STAT_NODES( cur2 ) + V_NODEIDS( cur1 ) + V_NODEIDS( cur2 ) * 3;
		ADDLINKS( cur1, cur3 );
		ADDLINKWS( cur2, cur1, 4.5 );
		ADDLINKS( cur3, cur2 );

		if ( h == 1 )						// copy the region: its agents will not be nodes
		{
			cur4 = ADDOBJ_EXS( PARENT, "Region", p );
			WRITES( cur4, "regionId", 4 );
		}
	break;

	case 4:
		if ( h == 4 )						// the copied region
		{
			v[1] = STAT_NET( "Agent" );
			v[9] += v[1] != v[1] ? 777 : 1;
			v[2] = INIT_NET( "Agent", "STAR", 0, 0, 0 );	// all existing objects become nodes
			STAT_NET( "Agent" );
			v[9] += v[0] + 3 * v[1] + 5 * v[2];
		}
	break;

	case 5:
		sprintf( nm, "netsops_r%d", h );

		if ( h == 3 )						// remove the network and rebuild it (fewer nodes than objects)
		{
			DELETE_NET( "Agent" );
			v[1] = STAT_NET( "Agent" );
			v[9] += v[1] != v[1] ? 888 : 1;
			v[2] = INIT_NET( "Agent", "RANDOM-UNDIR", 10, 14, 0 );
			STAT_NET( "Agent" );
			v[9] += 10 * v[0] + v[2];
		}
		else
			SHUFFLE_NET( "Agent" );

		v[9] += SAVE_NET( "Agent", nm );
	break;

	case 6:
		if ( h == 2 || h == 1 )
		{
			cur = SEARCH_NODE( "Agent", h == 2 ? 3 : 5 );
			v[9] += cur == NULL ? 4000 : 1;

			if ( cur != NULL )
			{
				CYCLE( cur1, "Agent" )		// remove the links to the node first
					for ( curl = cur1->node->first; curl != NULL; curl = curl1 )
					{
						curl1 = curl->next;
						if ( LINKTO( curl ) == cur )
							DELETE_LINK( curl );
					}

				if ( h == 2 )
					DELETE( cur );			// delete the object, node and links
				else
					DELETE_NODES( cur );	// delete only the node (network data)

				STAT_NET( "Agent" );
				v[9] += v[0] + v[1];
			}
		}
	break;

	case 7:
		if ( h == 1 )						// find the object without network data and make it a node again
		{
			cur = SEARCH_NODE( "Agent", 5 );
			v[9] += cur == NULL ? 5000 : 1;

			CYCLE( cur, "Agent" )
				if ( cur->node == NULL )
					break;

			if ( cur != NULL )
				ADDNODES( cur, 99, "revived" );
		}

		sprintf( nm, "netsops_r%d", h );
		v[9] += SNAP_NET( "Agent", nm );
	break;

	case 8:
		sprintf( nm, "netsops_r%d", h );
		v[9] += SNAP_NET( "Agent", nm );

		if ( h == 4 )						// delete the last three agents of the copied region
			for ( i = 0; i < 3; ++i )
			{
				CYCLE( cur, "Agent" )		// find the last one
					cur1 = cur;

				CYCLE( cur, "Agent" )		// remove the links to it first
					for ( curl = cur->node->first; curl != NULL; curl = curl1 )
					{
						curl1 = curl->next;
						if ( LINKTO( curl ) == cur1 )
							DELETE_LINK( curl );
					}

				DELETE( cur1 );
				STAT_NET( "Agent" );
				v[9] += v[0] * 10 + v[1];
			}
	break;
}

RESULT( v[9] )


EQUATION( "AgOps" )
/*
Operations of a node on its links and on other nodes
*/

if ( p->node == NULL )						// not a node
	END_EQUATION( -1 )

char nm[ 100 ];

v[9] = 0;

switch ( t )
{
	case 2:									// new links and drawing probabilities
		cur = RNDDRAW_NODES( PARENT, "Agent" );

		if ( cur != p && SEARCH_LINK( V_NODEIDS( cur ) ) == NULL )
		{
			curl = ADDLINKW( cur, t + 0.5 );
			curl1 = ADDLINKWS( cur, p, 2 * t );
			DRAWPROB_LINK( curl, 3 );
			v[9] += V_LINK( curl ) + V_LINK( curl1 ) + STAT_NODES( cur );
		}

		DRAWPROB_NODE( 1 + 2 * RND );
	break;

	case 3:									// draw, change and delete links
		curl = RNDDRAW_LINK;

		if ( curl != NULL )
		{
			v[9] += V_LINK( curl );
			WRITE_LINK( curl, V_LINK( curl ) * 2 );
			cur = LINKTO( curl );
			curl2 = SEARCH_LINKS( cur, V_NODEID );	// reverse link, if any

			if ( curl2 != NULL )
				v[9] += 100 + V_LINK( curl2 ) + ( LINKFROM( curl2 ) == cur ? 1000 : 0 );

			if ( ( (int) V_NODEID ) % 5 == 0 )
				DELETE_LINK( curl );
		}
	break;

	case 4:									// names and ids, replacing the node data
		if ( ( (int) V_NODEID ) % 2 == 0 )
			WRITE_NODEID( V_NODEID + 1000 );

		sprintf( nm, "ag%d_%d", (int) V_NODEID, (int) t );
		WRITE_NODENAME( nm );
		v[9] += strlen( V_NODENAME );

		if ( ( (int) V_NODEID ) % 7 == 0 )
		{
			ADDNODE( V_NODEID, "reborn" );	// discards the links, keeps the serial number
			v[9] += STAT_NODE + strlen( V_NODENAME );
		}
	break;

	case 5:									// links to nodes of this and of another region
		cur = RNDDRAW_NODES( PARENT, "Agent" );
		DRAWPROB_NODES( cur, ( (int) V_NODEIDS( cur ) ) % 3 + 1 );
		curl = ADDLINKS( p, cur );
		v[9] += curl == NULL ? 1 : V_LINK( curl ) + 2;

		if ( p->up->next != NULL )
		{
			cur2 = SEARCH_NODES( p->up->next, "Agent", 1 );
			curl2 = cur2 == NULL ? NULL : ADDLINKS( p, cur2 );
			v[9] += curl2 == NULL ? 77 : 88;
		}
	break;

	case 7:									// unweighted links
		cur = RNDDRAW_NODES( PARENT, "Agent" );

		if ( cur != p )
		{
			curl = ADDLINK( cur );
			v[9] += V_LINK( curl ) + 1;
		}
	break;

	case 9:									// delete the node data of some agents, after the links to them
		if ( ( (int) V_NODEID ) % 3 == 0 )
		{
			CYCLES( PARENT, cur, "Agent" )
				if ( cur->node != NULL )
					for ( curl = cur->node->first; curl != NULL; curl = curl1 )
					{
						curl1 = curl->next;
						if ( LINKTO( curl ) == p )
							DELETE_LINK( curl );
					}

			v[9] += STAT_NODE;
			DELETE_NODE;
		}
	break;

	case 6:									// cycle over the links of a neighbour
		curl = p->node->first;

		if ( curl != NULL )
		{
			cur = LINKTO( curl );
			CYCLE_LINKS( cur, curl1 )
				v[9] += V_LINK( curl1 ) + V_NODEIDS( LINKTO( curl1 ) );
		}
	break;
}

RESULT( v[9] )


EQUATION( "AgSig" )
/*
Order-sensitive signature of the node: its id, name and the sequence of
link destination ids and weights
*/

if ( p->node == NULL )
	END_EQUATION( -1 )

v[0] = V_NODEID * 1000003;
v[1] = 0;

CYCLE_LINK( curl )
{
	v[1]++;
	v[0] += v[1] * V_NODEIDS( LINKTO( curl ) ) + V_LINK( curl ) * 0.37;
}

if ( V_NODENAME != NULL )
	v[0] += strlen( V_NODENAME ) * 31;

RESULT( v[0] + STAT_NODE / 7 )


MODELEND


void close_sim( void )
{
}
