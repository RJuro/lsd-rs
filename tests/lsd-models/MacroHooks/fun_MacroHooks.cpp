#include "fun_head.h"

#include <vector>
using namespace std;

struct wext { double skill; vector<double> hist; wext( double s ) { skill = s; } };
struct fext { double total; int n; fext( double a, int b ) { total = a; n = b; } };

MODELBEGIN

EQUATION("Hist")

RESULT(RND)

EQUATION("Stat")
v[0] = V("Hist");
EXEC_EXT(wext, hist, push_back, v[0]);
WRITE_EXT(wext, skill, V_EXT(wext, skill) * 0.9 + 0.1 * v[0]);
WRITE("Rec", V_EXT(wext, skill));
WRITE("Rec", EXT(wext).hist.size());
WRITE("Rec", VS(PARENT, "Level"));
WRITE("Rec", VS(GRANDPARENT, "Top"));
WRITE("Rec", VS(ROOT, "Top"));
WRITE("Rec", strcmp(NAME, "Worker") == 0);
WRITE("Rec", NEXT == NULL);
WRITE("Rec", VS(SHOOK, "Level"));
WRITE("Rec", LAST_T);
if (COUNT_HOOK == 0) ADDHOOK(1);
WRITE_HOOK(0, NEXT);
cur = HOOK(0); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
RESULT(V_EXT(wext, skill))

EQUATION("Boss")
object *f1;
f1 = SEARCHS(PARENT, "Firm");
if (t == 1) {
ADDHOOK(3);
CYCLE(cur, "Worker") { ADDEXT_INITS(cur, wext, VS(cur, "Id") * 0.01); WRITE_SHOOKS(cur, p); }
ADDEXT_INIT(fext, 7.0, 0);
}
cur = ADDOBJ("Worker"); INCR("NextId", 1); WRITES(cur, "Id", V("NextId"));
ADDEXT_INITS(cur, wext, VS(cur, "Id") * 0.01); WRITE_SHOOKS(cur, p);
cur1 = cur;
if (COUNT("Worker") > 5) { cur = SEARCH("Worker"); DELETE(cur); }
WRITE_HOOK(0, SEARCH("Worker")); WRITE_HOOK(1, cur1);
WRITE_HOOK(2, NEXTS(HOOK(0)));
cur = HOOK(0); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = HOOK(1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = HOOK(2); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
WRITE("Rec", COUNT_HOOK);
ADDHOOKS(f1, 4); WRITE_HOOKS(f1, 3, p);
WRITE("Rec", COUNT_HOOKS(f1));
cur = HOOKS(f1, 0); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
WRITE("Rec", VS(HOOKS(f1, 3), "Level"));
WRITE_SHOOK(cur1);
cur = SHOOK; WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
cur = SHOOKS(f1); WRITE("Rec", cur == NULL ? -1 : VS(cur, "Id"));
WRITE_SHOOKS(f1, NULL); WRITE_SHOOK(NULL);
cur = SHOOK; WRITE("Rec", cur == NULL);
if (t == 5) { ADDHOOK(6); WRITE("Rec", COUNT_HOOK); ADDHOOK(2); WRITE("Rec", COUNT_HOOK); ADDHOOK(3); WRITE("Rec", COUNT_HOOK); WRITE_HOOK(0, NULL); }
WRITE("Rec", HOOK(0) == NULL);
WRITE_HOOK(0, SEARCH("Worker"));
WRITE("Rec", VS(PARENT, "Top"));
WRITE("Rec", VS(PARENTS(cur1), "Level"));
WRITE("Rec", VS(GRANDPARENTS(cur1), "Top"));
WRITE("Rec", NEXTS(cur1) == NULL);
WRITE("Rec", NAMES(cur1)[0]);
WRITE("Rec", strlen(NAME));
WRITE("Rec", VS(ROOT, "Top"));
WRITE_EXT(fext, total, V_EXT(fext, total) + SUM("Stat")); WRITE_EXT(fext, n, V_EXT(fext, n) + 1);
WRITE("Rec", V_EXT(fext, total));
WRITE("Rec", V_EXT(fext, n));
WRITE("Rec", EXTS(f1, fext).total);
WRITE("Rec", V_EXTS(cur1, wext, skill));
WRITE("Rec", EXEC_EXTS(cur1, wext, hist, size));
ADDEXT_INITS(cur1, wext, 5.5); WRITE_EXTS(cur1, wext, skill, 6.5);
WRITE("Rec", V_EXTS(cur1, wext, skill));
DELETE_EXTS(cur1, wext); ADDEXT_INITS(cur1, wext, 1.5);
if (t == 3) DELETE_EXT(fext);  if (t == 3) ADDEXT_INIT(fext, 100.0, 0);
RESULT(T)


MODELEND

void close_sim(void)
{
}
