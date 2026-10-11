#include "fun_head.h"


MODELBEGIN

EQUATION("A")

RESULT(VL("A",1)+RND)

EQUATION("B")

RESULT(V("A")*2+RND)

EQUATION("W")

RESULT(VL("W",1)*0.5+RND)

EQUATION("Z")

RESULT(V("W")*2+RND)

EQUATION("Dyn")
object *w, *ex, *f1;
USE_NAN; DEFAULT_RESULT(0.5);
f1 = SEARCHS(PARENT, "Firm");
v[0] = V("A");
WRITE("Rec", LAST_CALC("A"));
WRITE("Rec", RECALC("A"));
WRITE("Rec", LAST_CALC("A"));
WRITE("Rec", V("A"));
WRITE("Rec", VL("A", 1));
WRITE("Rec", VL("A", 2));
WRITE("Rec", LAST_CALCS(f1, "B"));
WRITE("Rec", RECALCS(f1, "B"));
UPDATE;
WRITE("Rec", LAST_CALC("B"));
WRITE("Rec", RECALCS(f1, "B"));
WRITE("Rec", VS(f1, "B"));
UPDATE_REC;
WRITE("Rec", SUM("W"));
w = SEARCH("Worker");
WRITE("Rec", LAST_CALCS(w, "Z"));
WRITE("Rec", RECALCS(w, "W"));
WRITE("Rec", VS(w, "W"));
WRITE("Rec", VLS(w, "W", 1));
WRITE("Rec", VLS(w, "W", 2));
WRITE("Rec", VS(w, "Z"));
UPDATES(w);
WRITE("Rec", LAST_CALCS(w, "W"));
UPDATE_RECS(f1);
ex = SEARCH("Worker");
w = ADDNOBJ("Worker", 2); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJL("Worker", 1, t - 1); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJL("Worker", 1, 0); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDOBJL("Worker", t - 1); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJ_EX("Worker", 1, ex); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJ_EXL("Worker", 2, ex, t); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDOBJ_EXL("Worker", ex, t); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJS(f1, "Worker", 1); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJLS(f1, "Worker", 1, t - 1); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJ_EXS(f1, "Worker", 1, ex); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDNOBJ_EXLS(f1, "Worker", 1, ex, t); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDOBJLS(f1, "Worker", t - 1); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
w = ADDOBJ_EXLS(f1, "Worker", ex, t); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
ADDNOBJ("Worker", 0);
WRITE("Rec", COUNT("Worker"));
WRITE("Rec", SUM_CND("Z", "Born", "==", t));
CYCLE_SAFE(cur, "Worker") if (t - VS(cur, "Born") > 3) DELETE(cur);
WRITE("Rec", COUNT("Worker"));
NO_SEARCH;
WRITE("Rec", V("A"));
WRITE("Rec", LAST_CALC("B"));
WRITE("Rec", SUM("W"));
WRITE("Rec", COUNT("Worker"));
USE_SEARCH;
if (V("Zero") == 1 && fmod(t, 5) == 0) {
USE_ZERO_INSTANCE; CYCLE_SAFE(cur, "Worker") DELETE(cur);
WRITE("Rec", COUNT("Worker"));
WRITE("Rec", SUM("W"));
WRITE("Rec", MAX("W"));
WRITE("Rec", AVE_CND("W", "Born", ">", 0));
WRITE("Rec", SEARCH("Worker") == NULL);
WRITE("Rec", RNDDRAW_FAIR("Worker") == NULL);
WRITE("Rec", COUNT_ALL("Worker"));
w = ADDNOBJ("Worker", 3); for (cur = w; cur != NULL; cur = cur->next) { INCR("NextId", 1); WRITES(cur, "Id", V("NextId")); WRITES(cur, "Born", t); } WRITE("Rec", VS(w, "W")); WRITE("Rec", LAST_CALCS(w, "W"));
NO_ZERO_INSTANCE; }
WRITE("Rec", COUNT("Worker"));
if (t == 12 && V("Zero") == 2) ABORT;
RESULT(T)


MODELEND

void close_sim(void)
{
}
