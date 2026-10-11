#include "fun_head.h"


MODELBEGIN

EQUATION("X")

RESULT(rnd_integer(0,6))

EQUATION("Y")

RESULT(RND*10)

EQUATION("N")

RESULT(-1-rnd_integer(0,4))

EQUATION("Zs")

RESULT(VL("Zs",1)+RND-0.5)

EQUATION("RStats")
USE_NAN;
object *g2;
g2 = NEXTS(SEARCH("Group"));
v[0]=WHTAVE("X", "Y"); WRITE("Rec",v[0]);
v[0]=SUM("X"); WRITE("Rec",v[0]);
v[0]=COUNT("Item"); WRITE("Rec",v[0]);
v[0]=AVE("Y"); WRITE("Rec",v[0]);
v[0]=SUMS(g2, "X"); WRITE("Rec",v[0]);
v[0]=MEDS(g2, "Y"); WRITE("Rec",v[0]);
v[0]=SD_CNDS(g2, "Y", "Cat", "==", 2); WRITE("Rec",v[0]);
v[0]=PERC("Y", 0.9); WRITE("Rec",v[0]);
v[0]=MIN_CND("X", "Cat", ">=", 2); WRITE("Rec",v[0]);
STATS(g2, "X"); for(i=0;i<7;++i) WRITE("Rec",v[i]);
RESULT(T)

EQUATION("GStats")
USE_NAN;
USE_SAVED;
object *g1, *g2;
double w[6] = { 1, 2, 0.5, 3, 1.5, 2 };
g1 = SEARCHS(PARENT, "Group");
v[0]=SUM("X"); WRITE("Rec",v[0]);
v[0]=SUML("X", 1); WRITE("Rec",v[0]);
v[0]=SUMS(g1, "X"); WRITE("Rec",v[0]);
v[0]=SUMLS(g1, "X", 2); WRITE("Rec",v[0]);
v[0]=SUM_CND("X", "Cat", "==", 2.0); WRITE("Rec",v[0]);
v[0]=SUM_CNDL("X", "Y", ">", 4.5, 1); WRITE("Rec",v[0]);
v[0]=SUM_CNDS(g1, "X", "X", "<=", 3.0); WRITE("Rec",v[0]);
v[0]=SUM_CNDLS(g1, "X", "Cat", "!=", 1.0, 2); WRITE("Rec",v[0]);
v[0]=MAX("Y"); WRITE("Rec",v[0]);
v[0]=MAXL("Y", 1); WRITE("Rec",v[0]);
v[0]=MAXS(g1, "Y"); WRITE("Rec",v[0]);
v[0]=MAXLS(g1, "Y", 2); WRITE("Rec",v[0]);
v[0]=MAX_CND("Y", "Cat", "!=", 1.0); WRITE("Rec",v[0]);
v[0]=MAX_CNDL("Y", "X", "NNAN", 0.0, 1); WRITE("Rec",v[0]);
v[0]=MAX_CNDS(g1, "Y", "Cat", "GT", 1.0); WRITE("Rec",v[0]);
v[0]=MAX_CNDLS(g1, "Y", "Y", "LT", 5.0, 2); WRITE("Rec",v[0]);
v[0]=MIN("X"); WRITE("Rec",v[0]);
v[0]=MINL("X", 1); WRITE("Rec",v[0]);
v[0]=MINS(g1, "X"); WRITE("Rec",v[0]);
v[0]=MINLS(g1, "X", 2); WRITE("Rec",v[0]);
v[0]=MIN_CND("X", "Y", "LT", 5.0); WRITE("Rec",v[0]);
v[0]=MIN_CNDL("X", "X", "GE", 2.0, 1); WRITE("Rec",v[0]);
v[0]=MIN_CNDS(g1, "X", "Cat", "LE", 2.0); WRITE("Rec",v[0]);
v[0]=MIN_CNDLS(g1, "X", "X", "EQ", 3.0, 2); WRITE("Rec",v[0]);
v[0]=AVE("X"); WRITE("Rec",v[0]);
v[0]=AVEL("X", 1); WRITE("Rec",v[0]);
v[0]=AVES(g1, "X"); WRITE("Rec",v[0]);
v[0]=AVELS(g1, "X", 2); WRITE("Rec",v[0]);
v[0]=AVE_CND("X", "X", "EQ", 3.0); WRITE("Rec",v[0]);
v[0]=AVE_CNDL("X", "X", "NE", 3.0, 1); WRITE("Rec",v[0]);
v[0]=AVE_CNDS(g1, "X", "Cat", "=", 3.0); WRITE("Rec",v[0]);
v[0]=AVE_CNDLS(g1, "X", "Cat", "==", 99.0, 2); WRITE("Rec",v[0]);
v[0]=WHTAVE("X", "W"); WRITE("Rec",v[0]);
v[0]=WHTAVEL("X", "W", 1); WRITE("Rec",v[0]);
v[0]=WHTAVES(g1, "X", "W"); WRITE("Rec",v[0]);
v[0]=WHTAVELS(g1, "X", "W", 2); WRITE("Rec",v[0]);
v[0]=WHTAVE_CND("X", "W", "Cat", "==", 99.0); WRITE("Rec",v[0]);
v[0]=WHTAVE_CNDL("X", "W", "Y", ">=", 0.5, 1); WRITE("Rec",v[0]);
v[0]=WHTAVE_CNDS(g1, "X", "W", "X", "<", 4.0); WRITE("Rec",v[0]);
v[0]=WHTAVE_CNDLS(g1, "X", "W", "Cat", "==", 2.0, 2); WRITE("Rec",v[0]);
v[0]=SD("Y"); WRITE("Rec",v[0]);
v[0]=SDL("Y", 1); WRITE("Rec",v[0]);
v[0]=SDS(g1, "Y"); WRITE("Rec",v[0]);
v[0]=SDLS(g1, "Y", 2); WRITE("Rec",v[0]);
v[0]=SD_CND("Y", "Cat", "==", 2.0); WRITE("Rec",v[0]);
v[0]=SD_CNDL("Y", "Y", ">", 4.5, 1); WRITE("Rec",v[0]);
v[0]=SD_CNDS(g1, "Y", "X", "<=", 3.0); WRITE("Rec",v[0]);
v[0]=SD_CNDLS(g1, "Y", "Cat", "!=", 1.0, 2); WRITE("Rec",v[0]);
v[0]=MED("X"); WRITE("Rec",v[0]);
v[0]=MEDL("X", 1); WRITE("Rec",v[0]);
v[0]=MEDS(g1, "X"); WRITE("Rec",v[0]);
v[0]=MEDLS(g1, "X", 2); WRITE("Rec",v[0]);
v[0]=MED_CND("X", "Cat", "!=", 1.0); WRITE("Rec",v[0]);
v[0]=MED_CNDL("X", "X", "NNAN", 0.0, 1); WRITE("Rec",v[0]);
v[0]=MED_CNDS(g1, "X", "Cat", "GT", 1.0); WRITE("Rec",v[0]);
v[0]=MED_CNDLS(g1, "X", "Y", "LT", 5.0, 2); WRITE("Rec",v[0]);
v[0]=PERC("Y", 0.3); WRITE("Rec",v[0]);
v[0]=PERCL("Y", 0.3, 1); WRITE("Rec",v[0]);
v[0]=PERCS(g1, "Y", 0.3); WRITE("Rec",v[0]);
v[0]=PERCLS(g1, "Y", 0.3, 2); WRITE("Rec",v[0]);
v[0]=PERC_CND("Y", 0.3, "Y", "LT", 5.0); WRITE("Rec",v[0]);
v[0]=PERC_CNDL("Y", 0.3, "X", "GE", 2.0, 1); WRITE("Rec",v[0]);
v[0]=PERC_CNDS(g1, "Y", 0.3, "Cat", "LE", 2.0); WRITE("Rec",v[0]);
v[0]=PERC_CNDLS(g1, "Y", 0.3, "X", "EQ", 3.0, 2); WRITE("Rec",v[0]);
v[0]=COUNT("Item"); WRITE("Rec",v[0]);
v[0]=COUNTS(g1, "Item"); WRITE("Rec",v[0]);
v[0]=COUNT_CND("Item", "X", "EQ", 3.0); WRITE("Rec",v[0]);
v[0]=COUNT_CNDL("Item", "X", "NE", 3.0, 1); WRITE("Rec",v[0]);
v[0]=COUNT_CNDS(g1, "Item", "Cat", "=", 3.0); WRITE("Rec",v[0]);
v[0]=COUNT_CNDLS(g1, "Item", "Cat", "==", 99.0, 2); WRITE("Rec",v[0]);
v[0]=COUNT_ALL("Item"); WRITE("Rec",v[0]);
v[0]=COUNT_ALLS(g1, "Item"); WRITE("Rec",v[0]);
v[0]=COUNT_ALL_CND("Item", "Cat", "==", 99.0); WRITE("Rec",v[0]);
v[0]=COUNT_ALL_CNDL("Item", "Y", ">=", 0.5, 1); WRITE("Rec",v[0]);
v[0]=COUNT_ALL_CNDS(g1, "Item", "X", "<", 4.0); WRITE("Rec",v[0]);
v[0]=COUNT_ALL_CNDLS(g1, "Item", "Cat", "==", 2.0, 2); WRITE("Rec",v[0]);
STAT("X"); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STATL("Y", 1); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STATS(g1, "X"); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STATLS(g1, "Y", 2); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT_CND("X", "Cat", "GT", 1.0); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT_CNDL("Y", "Y", "LT", 5.0, 1); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT_CNDS(g1, "X", "X", "GE", 2.0); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT_CNDLS(g1, "Y", "Cat", "LE", 2.0, 2); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT("N"); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT_CND("N", "Cat", "==", 2); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT_CND("N", "Cat", "==", 99); for(i=0;i<7;++i) WRITE("Rec",v[i]);
v[0]=MAVE("Zs", 4); WRITE("Rec",v[0]);
v[0]=MAVEL("Zs", 3, 1); WRITE("Rec",v[0]);
v[0]=MAVES(g1, "Zs", 5); WRITE("Rec",v[0]);
v[0]=MAVELS(g1, "Zs", 2, 2); WRITE("Rec",v[0]);
v[0]=MAVE("Zs", -2); WRITE("Rec",v[0]);
v[0]=WHTMAVE("Zs", 3, w); WRITE("Rec",v[0]);
v[0]=WHTMAVEL("Zs", 4, w, 1); WRITE("Rec",v[0]);
v[0]=WHTMAVES(g1, "Zs", 3, w); WRITE("Rec",v[0]);
v[0]=WHTMAVELS(g1, "Zs", 5, w, 2); WRITE("Rec",v[0]);
v[0]=MAX("N"); WRITE("Rec",v[0]);
v[0]=MIN("N"); WRITE("Rec",v[0]);
v[0]=MED("N"); WRITE("Rec",v[0]);
v[0]=SD("N"); WRITE("Rec",v[0]);
v[0]=COUNT_ALL("Group"); WRITE("Rec",v[0]);
v[0]=COUNT_ALL_CND("Group", "Zs", ">", 0.0); WRITE("Rec",v[0]);
NO_SEARCH;
v[0]=SUM("X"); WRITE("Rec",v[0]);
v[0]=COUNT("Item"); WRITE("Rec",v[0]);
v[0]=AVE_CND("Y", "Cat", "==", 1); WRITE("Rec",v[0]);
v[0]=MAXL("X", 1); WRITE("Rec",v[0]);
USE_SEARCH;
v[0] = 0;
RESULT(T)

EQUATION("IStats")
USE_NAN;
v[0]=COUNT("Item"); WRITE("Rec",v[0]);
v[0]=COUNT_ALL("Item"); WRITE("Rec",v[0]);
v[0]=COUNT_ALL_CND("Item", "Cat", ">", 1); WRITE("Rec",v[0]);
v[0]=COUNT_CND("Item", "X", "<", 3); WRITE("Rec",v[0]);
v[0]=SUM("X"); WRITE("Rec",v[0]);
v[0]=AVE_CND("Y", "Cat", "!=", 2); WRITE("Rec",v[0]);
v[0]=MAX_CNDL("X", "Cat", "==", 3, 1); WRITE("Rec",v[0]);
STAT("X"); for(i=0;i<7;++i) WRITE("Rec",v[i]);
STAT_CND("Y", "Cat", "<", 3); for(i=0;i<7;++i) WRITE("Rec",v[i]);
RESULT(T)


MODELEND

void close_sim(void)
{
}
