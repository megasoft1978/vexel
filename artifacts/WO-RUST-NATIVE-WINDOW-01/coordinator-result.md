# Accepted native-window SOURCE R2

Complete real Apple native window source is integrated: weak generation registry,
NSView/NSWindowDelegate/NSTextInputClient callbacks, bounded event pump/timers and
main runloop wakeups, Quartz geometry/appearance/full cursors/material gate,
owned IME projection and UTF16/snapshot queries, callback faults and safe teardown.
Initial incomplete R1 is preserved. Source implementation used named SolMedium;
independent LunaHigh review found reservedqueue over-admission, repaired by narrow
same-session LunaLow plus240ordinary/16reserved/17th-control regression.
Independent recheck ACCEPT. Initial coordinator bridge audit expected an appended
module declaration, while valid candidate inserted it beside existing modules;
exact original text after removing only authorized declaration/exports passes.

Fresh main lockedoffline check and41focused tests pass:29laneunit+1laneintegration
+5oldboundary+6compilefail, including7original boundary proofs byte-preserved.
603legacy/157oracles/source-scope/HEAD/index/exact reviewed source audit pass.
This accepts source only, not liveGUI/mac13/timer/IME/display/cursor/native
teardown runtime. Menus/files/dialogs/AX/shaping/preferences/welcome/Metal/app/MCP
and final tested release/authentic recorded demonstration remain required.
