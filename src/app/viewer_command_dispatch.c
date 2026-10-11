#include "viewer_internal.h"
#include "viewer_commands.h"

void viewer_apply_command(viewer_app *app, viewer_command_id command)
{
    if (app == NULL || app->editor == NULL || viewer_command_find(command) == NULL)
        return;
    switch (command) {
        case VIEWER_COMMAND_SEARCH:
            (void)viewer_editor_open_command_palette(app->editor);
            app->frame_dirty = true;
            return;
        case VIEWER_COMMAND_TOOL_SELECT:
            (void)viewer_editor_set_tool(app->editor, UI_ICON_SELECT); break;
        case VIEWER_COMMAND_TOOL_PAN:
            (void)viewer_editor_set_tool(app->editor, UI_ICON_PAN); break;
        case VIEWER_COMMAND_TOOL_ZOOM:
            (void)viewer_editor_set_tool(app->editor, UI_ICON_ZOOM); break;
        case VIEWER_COMMAND_TOOL_RECTANGLE:
            (void)viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_RECT); break;
        case VIEWER_COMMAND_TOOL_ELLIPSE:
            (void)viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_ELLIPSE); break;
        case VIEWER_COMMAND_TOOL_LINE:
            (void)viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_LINE); break;
        case VIEWER_COMMAND_TOOL_POLYGON:
            (void)viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_POLYGON); break;
        case VIEWER_COMMAND_TOOL_POLYLINE:
            (void)viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_POLYLINE); break;
        case VIEWER_COMMAND_TOOL_PEN:
            (void)viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_PEN); break;
        case VIEWER_COMMAND_TOOL_TEXT:
            (void)viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_TEXT); break;
        case VIEWER_COMMAND_TOOL_NODE:
            viewer_apply_synthetic_key(app, 45U, 0U, "n", 1U); return;
        case VIEWER_COMMAND_UNDO:
            viewer_apply_synthetic_key(app, 6U, PLATFORM_MODIFIER_COMMAND, "z", 1U); return;
        case VIEWER_COMMAND_REDO:
            viewer_apply_synthetic_key(app, 6U,
                PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U); return;
        case VIEWER_COMMAND_UNDO_HISTORY:
            (void)viewer_editor_open_history(app->editor);
            app->frame_dirty = true;
            return;
        case VIEWER_COMMAND_SELECT_ALL:
            viewer_apply_synthetic_key(app, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U); return;
        case VIEWER_COMMAND_COPY:
            viewer_apply_synthetic_key(app, 8U, PLATFORM_MODIFIER_COMMAND, "c", 1U); return;
        case VIEWER_COMMAND_CUT:
            viewer_apply_synthetic_key(app, 7U, PLATFORM_MODIFIER_COMMAND, "x", 1U); return;
        case VIEWER_COMMAND_PASTE:
            viewer_apply_synthetic_key(app, 9U, PLATFORM_MODIFIER_COMMAND, "v", 1U); return;
        case VIEWER_COMMAND_PASTE_IN_PLACE:
            viewer_apply_synthetic_key(app, 9U,
                PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "v", 1U); return;
        case VIEWER_COMMAND_DUPLICATE:
            viewer_apply_synthetic_key(app, 2U, PLATFORM_MODIFIER_COMMAND, "d", 1U); return;
        case VIEWER_COMMAND_DELETE:
            viewer_apply_synthetic_key(app, 51U, 0U, NULL, 0U); return;
        case VIEWER_COMMAND_GROUP:
            viewer_apply_synthetic_key(app, 5U, PLATFORM_MODIFIER_COMMAND, "g", 1U); return;
        case VIEWER_COMMAND_UNGROUP:
            viewer_apply_synthetic_key(app, 5U,
                PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "g", 1U); return;
        case VIEWER_COMMAND_BRING_FORWARD:
            viewer_apply_synthetic_key(app, 30U, PLATFORM_MODIFIER_COMMAND, "]", 1U); return;
        case VIEWER_COMMAND_SEND_BACKWARD:
            viewer_apply_synthetic_key(app, 33U, PLATFORM_MODIFIER_COMMAND, "[", 1U); return;
        case VIEWER_COMMAND_BRING_TO_FRONT:
            viewer_apply_synthetic_key(app, 30U,
                PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "]", 1U); return;
        case VIEWER_COMMAND_SEND_TO_BACK:
            viewer_apply_synthetic_key(app, 33U,
                PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "[", 1U); return;
        case VIEWER_COMMAND_ALIGN_LEFT:
        case VIEWER_COMMAND_ALIGN_CENTER:
        case VIEWER_COMMAND_ALIGN_RIGHT:
        case VIEWER_COMMAND_ALIGN_TOP:
        case VIEWER_COMMAND_ALIGN_MIDDLE:
        case VIEWER_COMMAND_ALIGN_BOTTOM: {
            viewer_editor_alignment alignment = VIEWER_EDITOR_ALIGN_LEFT;
            if (command == VIEWER_COMMAND_ALIGN_CENTER)
                alignment = VIEWER_EDITOR_ALIGN_HORIZONTAL_CENTER;
            else if (command == VIEWER_COMMAND_ALIGN_RIGHT)
                alignment = VIEWER_EDITOR_ALIGN_RIGHT;
            else if (command == VIEWER_COMMAND_ALIGN_TOP)
                alignment = VIEWER_EDITOR_ALIGN_TOP;
            else if (command == VIEWER_COMMAND_ALIGN_MIDDLE)
                alignment = VIEWER_EDITOR_ALIGN_VERTICAL_CENTER;
            else if (command == VIEWER_COMMAND_ALIGN_BOTTOM)
                alignment = VIEWER_EDITOR_ALIGN_BOTTOM;
            viewer_process_editor_result(app, viewer_editor_align_selection(app->editor,
                &app->document.shape, alignment));
            return;
        }
        case VIEWER_COMMAND_DISTRIBUTE_HORIZONTAL:
        case VIEWER_COMMAND_DISTRIBUTE_VERTICAL:
            viewer_process_editor_result(app, viewer_editor_distribute_selection(app->editor,
                &app->document.shape, command == VIEWER_COMMAND_DISTRIBUTE_HORIZONTAL ?
                    VIEWER_EDITOR_DISTRIBUTE_HORIZONTAL : VIEWER_EDITOR_DISTRIBUTE_VERTICAL));
            return;
        case VIEWER_COMMAND_FIT_WIDTH: viewer_apply_action(app, PLATFORM_ACTION_FIT_WIDTH); return;
        case VIEWER_COMMAND_FIT_WINDOW: viewer_apply_action(app, PLATFORM_ACTION_FIT_WINDOW); return;
        case VIEWER_COMMAND_ACTUAL_SIZE: viewer_apply_action(app, PLATFORM_ACTION_ACTUAL_SIZE); return;
        case VIEWER_COMMAND_ZOOM_IN: viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_IN); return;
        case VIEWER_COMMAND_ZOOM_OUT: viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_OUT); return;
        case VIEWER_COMMAND_TOGGLE_SNAPPING:
            (void)viewer_editor_toggle_snapping(app->editor); break;
        case VIEWER_COMMAND_TOGGLE_CHECKERBOARD:
            viewer_toggle_checkerboard(app); return;
        case VIEWER_COMMAND_CYCLE_BACKGROUND: viewer_cycle_background(app); return;
        case VIEWER_COMMAND_TOGGLE_PIXEL_GRID: viewer_toggle_pixel_grid(app); return;
        case VIEWER_COMMAND_OPEN: (void)platform_window_open_panel(app->window); return;
        case VIEWER_COMMAND_SAVE: (void)viewer_save_document(app, false); return;
        case VIEWER_COMMAND_SAVE_AS: (void)viewer_save_document(app, true); return;
        case VIEWER_COMMAND_RELOAD: viewer_request_reload_or_open(app); return;
        case VIEWER_COMMAND_EXPORT_PNG: (void)viewer_export_png(app, false); return;
        case VIEWER_COMMAND_COPY_PNG: (void)viewer_export_png(app, true); return;
        case VIEWER_COMMAND_QUIT: viewer_request_close(app); return;
        case VIEWER_COMMAND_NONE:
        case VIEWER_COMMAND_COUNT:
        default: return;
    }
    app->frame_dirty = true;
}
