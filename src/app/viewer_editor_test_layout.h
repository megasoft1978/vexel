#ifndef VEXEL_APP_VIEWER_EDITOR_TEST_LAYOUT_H
#define VEXEL_APP_VIEWER_EDITOR_TEST_LAYOUT_H

#include "viewer_editor.h"

#include <stdio.h>

/* Writes the last painted editor nodes and their pointer hit regions as JSON. */
bool viewer_editor_test_layout_write_nodes(viewer_editor *editor, FILE *stream);

#endif
