// Reference for engine/src/rng.rs: LSD util.cpp's RNG functions, verbatim logic, mt19937 path (ran_gen_id = 2).
// usage: rng_ref SEED NDRAWS CALLS_FILE   (cycles through the calls file, one %.17g value per line)
#include <random>
#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>
using namespace std;

minstd_rand lc1;
mt19937 mt32;

template < class distr > double draw_gen( distr &d ) { return d( mt32 ); }

int rnd_int( int min, int max ) { uniform_int_distribution< int > distr( min, max ); return distr( lc1 ); }
double ran1( ) { double ran; uniform_real_distribution< double > distr( 0, 1 ); do ran = draw_gen( distr ); while ( ran == 0.0 ); return ran; }
double uniform( double min, double max ) { uniform_real_distribution< double > distr( min, max ); return draw_gen( distr ); }
double uniform_int( double min, double max ) { uniform_int_distribution< int > distr( ( long ) min, ( long ) max ); return draw_gen( distr ); }
double norm( double mean, double dev ) { if ( dev < 0 ) return mean; normal_distribution< double > distr( mean, dev ); return draw_gen( distr ); }
double lnorm( double mean, double dev ) { if ( dev < 0 ) return exp( mean ); lognormal_distribution< double > distr( mean, dev ); return draw_gen( distr ); }
double gamma( double alpha, double beta ) { if ( alpha <= 0 || beta <= 0 ) return 0.0; gamma_distribution< double > distr( alpha, beta ); return draw_gen( distr ); }
double bernoulli( double p ) { if ( p < 0 || p > 1 ) return p < 0 ? 0.0 : 1.0; bernoulli_distribution distr( p ); return draw_gen( distr ); }
double poisson( double mean ) { if ( mean < 0 ) return 0.0; poisson_distribution< int > distr( mean ); return draw_gen( distr ); }
double beta( double alpha, double beta )
{
	if ( alpha <= 0 || beta <= 0 ) return alpha < beta ? 0.0 : 1.0;
	gamma_distribution< double > distr1( alpha, 1.0 ), distr2( beta, 1.0 );
	double draw = draw_gen( distr1 );
	return draw / ( draw + draw_gen( distr2 ) );
}

struct Call { string op; double a, b; };

int main( int argc, char **argv )
{
	if ( argc < 4 ) return 1;
	unsigned seed = strtoul( argv[1], NULL, 10 );
	long n = atol( argv[2] );
	lc1.seed( seed );
	mt32.seed( seed );

	vector< Call > calls;
	ifstream f( argv[3] );
	string line;
	while ( getline( f, line ) )
	{
		if ( line.empty( ) || line[0] == '#' ) continue;
		istringstream is( line );
		string a, b;
		Call c;
		is >> c.op >> a >> b;
		c.a = strtod( a.c_str( ), NULL );
		c.b = strtod( b.c_str( ), NULL );
		calls.push_back( c );
	}

	for ( long i = 0; i < n; ++i )
	{
		const Call &c = calls[i % calls.size( )];
		double v;
		if ( c.op == "ran1" ) v = ran1( );
		else if ( c.op == "uniform" ) v = uniform( c.a, c.b );
		else if ( c.op == "uniform_int" ) v = uniform_int( c.a, c.b );
		else if ( c.op == "rnd_int" ) v = rnd_int( ( int ) c.a, ( int ) c.b );
		else if ( c.op == "norm" ) v = norm( c.a, c.b );
		else if ( c.op == "lnorm" ) v = lnorm( c.a, c.b );
		else if ( c.op == "gamma" ) v = gamma( c.a, c.b );
		else if ( c.op == "beta" ) v = beta( c.a, c.b );
		else if ( c.op == "poisson" ) v = poisson( c.a );
		else if ( c.op == "bernoulli" ) v = bernoulli( c.a );
		else return 2;
		printf( "%.17g\n", v );
	}
	return 0;
}
