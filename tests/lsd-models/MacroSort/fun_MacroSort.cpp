#include "fun_head.h"


MODELBEGIN

EQUATION("F")

RESULT(RND)

EQUATION("K")

RESULT(rnd_integer(0,9))

EQUATION("K2")

RESULT(RND)

EQUATION("Lk")

RESULT(rnd_integer(0,3))

EQUATION("Sorter")
object *g1;
g1 = SEARCHS(PARENT, "Group");
INIT_TSEARCH("Item");
SORT("Item", "F", "UP");
CYCLE(cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORT("Item", "K", "UP");
CYCLE(cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORT("Item", "K", "DOWN");
CYCLE(cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORTL("Item", "Lk", "UP", 1);
CYCLE(cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORT2("Item", "K", "K2", "UP");
CYCLE(cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORT2("Item", "K", "K2", "DOWN");
CYCLE(cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORT2L("Item", "Lk", "K2", "UP", 1);
CYCLE(cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORTS(g1, "Item", "K", "down");
CYCLES(g1, cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORTLS(g1, "Item", "Lk", "UP", 2);
CYCLES(g1, cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORT2S(g1, "Item", "K", "K2", "Down");
CYCLES(g1, cur, "Item") WRITE("Rec", VS(cur, "Id"));
SORT2LS(g1, "Item", "Lk", "K2", "up", 1);
CYCLES(g1, cur, "Item") WRITE("Rec", VS(cur, "Id"));
v[0] = COUNT("Item");
cur = TSEARCH("Item", 1); WRITE("Rec", VS(cur, "Id"));
cur = TSEARCH("Item", ceil(v[0] / 2)); WRITE("Rec", VS(cur, "Id"));
cur = TSEARCH("Item", v[0]); WRITE("Rec", VS(cur, "Id"));
cur = SEARCH("Item"); WRITE("Rec", SEARCH_INST(cur));
cur = TSEARCH("Item", v[0]); WRITE("Rec", SEARCH_INST(cur));
WRITE("Rec", SEARCH_INSTS(g1, TSEARCH("Item", 1)));
RESULT(T)


MODELEND

void close_sim(void)
{
}
