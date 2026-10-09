#include "ui_widgets.h"

#include <stdio.h>
#include <time.h>

#define UI_FOCUS_BENCH_MOVES 5000000U

int main(void)
{
    ui_tree tree;
    if (!ui_tree_init(&tree, UI_THEME_DARK)) return 1;
    const char label[] = "Focus target";
    for (uint32_t id = 2U; id <= UI_MAX_NODES; id++) {
        ui_widget_spec spec = {0};
        spec.id = id;
        spec.parent_id = UI_ROOT_ID;
        spec.axis = UI_AXIS_HORIZONTAL;
        spec.label = label;
        spec.label_len = sizeof(label) - 1U;
        spec.command_id = id;
        if (!ui_widget_add_button(&tree, &spec)) return 2;
    }
    if (!ui_focus_next(&tree, false)) return 3;
    clock_t start = clock();
    if (start == (clock_t)-1) return 4;
    for (uint32_t move = 0U; move < UI_FOCUS_BENCH_MOVES; move++) {
        if (!ui_focus_next(&tree, false)) return 5;
    }
    clock_t elapsed = clock() - start;
    if (elapsed <= 0) return 6;
    double milliseconds = (double)elapsed * 1000.0 / (double)CLOCKS_PER_SEC;
    (void)printf("{\"nodes\":%u,\"focus_moves\":%u,\"cpu_ms\":%.3f,\"ns_per_move\":%.1f}\n",
                 UI_MAX_NODES, UI_FOCUS_BENCH_MOVES, milliseconds,
                 milliseconds * 1000000.0 / (double)UI_FOCUS_BENCH_MOVES);
    return 0;
}
