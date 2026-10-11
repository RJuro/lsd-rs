#include "fun_head.h"


MODELBEGIN

EQUATION("P")

RESULT(RND+0.1)

EQUATION("Q")

RESULT(RND+0.1)

EQUATION("V")

RESULT(uniform_int(1,5))

EQUATION("Search")
object *g1; double n, tot;
g1 = SEARCHS(PARENT, "Pop");
n = COUNT("Agent");
WRITE("Rec", INIT_TSEARCH("Agent"));
v[0] = 1; cur = TSEARCH("Agent", v[0]); WRITE("Rec", VS(cur, "Id"));
v[0] = n; cur = TSEARCH("Agent", v[0]); WRITE("Rec", VS(cur, "Id"));
v[0] = ceil(n / 2); cur = TSEARCH("Agent", v[0]); WRITE("Rec", VS(cur, "Id"));
v[0] = uniform_int(1, n); cur = TSEARCH("Agent", v[0]); WRITE("Rec", VS(cur, "Id"));
v[0] = uniform_int(1, n); cur = TSEARCH("Agent", v[0]); WRITE("Rec", VS(cur, "Id"));
WRITE("Rec", INIT_TSEARCHT("Agent", n + 5));
cur = TSEARCH("Agent", n + 3); WRITE("Rec", VS(cur, "Id"));
WRITE("Rec", INIT_TSEARCHS(g1, "Agent"));
v[1] = COUNTS(g1, "Agent"); v[0] = uniform_int(1, v[1]); cur = TSEARCHS(g1, "Agent", v[0]); WRITE("Rec", VS(cur, "Id"));
WRITE("Rec", INIT_TSEARCHTS(g1, "Agent", 0));
cur = TSEARCHS(g1, "Agent", 1); WRITE("Rec", VS(cur, "Id"));
WRITE("Rec", INIT_TSEARCH_CND("V"));
cur = TSEARCH_CND("V", 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = TSEARCH_CND("V", 2); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = TSEARCH_CND("V", 3); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = TSEARCH_CND("V", 4); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = TSEARCH_CND("V", 5); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = TSEARCH_CND("V", 99); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
WRITE("Rec", INIT_TSEARCH_CNDS(g1, "V"));
v[0] = uniform_int(1, 5); cur = TSEARCH_CNDS(g1, "V", v[0]); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = TSEARCH_CNDS(g1, "V", 3); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
tot = SUM("P");
cur = RNDDRAW_TOT("Agent", "P", tot); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_TOT("Agent", "P", tot); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_TOT("Agent", "P", tot); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_TOTL("Agent", "Q", 1, SUML("Q", 1)); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_TOTS(g1, "Agent", "P", SUMS(g1, "P")); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_TOTLS(g1, "Agent", "Q", 2, SUMLS(g1, "Q", 2)); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW("Agent", "P"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAWL("Agent", "Q", 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAWS(g1, "Agent", "P"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAWLS(g1, "Agent", "Q", 2); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_FAIR("Agent"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_FAIRS(g1, "Agent"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW("Agent", "P"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAWL("Agent", "Q", 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAWS(g1, "Agent", "P"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAWLS(g1, "Agent", "Q", 2); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_FAIR("Agent"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_FAIRS(g1, "Agent"); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = RNDDRAW_FAIRS(g1, "Agent"); cur1 = RNDDRAW_FAIR("Agent");
WRITE("Rec", SEARCH_INST(cur));
WRITE("Rec", SEARCH_INSTS(g1, cur));
WRITE("Rec", SEARCH_INST(cur1));
WRITE("Rec", SEARCH_INSTS(cur1, cur1));
WRITE("Rec", SEARCH_INSTS(cur, cur1));
cur = RNDDRAW_FAIRS(g1, "Agent"); cur1 = RNDDRAW_FAIR("Agent");
WRITE("Rec", SEARCH_INST(cur));
WRITE("Rec", SEARCH_INSTS(g1, cur));
WRITE("Rec", SEARCH_INST(cur1));
WRITE("Rec", SEARCH_INSTS(cur1, cur1));
WRITE("Rec", SEARCH_INSTS(cur, cur1));
cur = RNDDRAW_FAIRS(g1, "Agent"); cur1 = RNDDRAW_FAIR("Agent");
WRITE("Rec", SEARCH_INST(cur));
WRITE("Rec", SEARCH_INSTS(g1, cur));
WRITE("Rec", SEARCH_INST(cur1));
WRITE("Rec", SEARCH_INSTS(cur1, cur1));
WRITE("Rec", SEARCH_INSTS(cur, cur1));
cur = SEARCH_CNDL("V", 3, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = SEARCH_CNDLS(g1, "V", 3, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = SEARCH_CNDL("V", 4, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = SEARCH_CNDLS(g1, "V", 4, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = SEARCH_CNDL("V", 5, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = SEARCH_CNDLS(g1, "V", 5, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
NO_SEARCH;
cur = SEARCH_CNDL("V", 2, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = SEARCH_CNDL("V", 99, 1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
WRITE("Rec", COUNT("Agent"));
USE_SEARCH;
RESULT(T)


MODELEND

void close_sim(void)
{
}
