#ifndef VEXEL_APP_PLATFORM_COMMAND_POLICY_H
#define VEXEL_APP_PLATFORM_COMMAND_POLICY_H

#include "platform/platform_window.h"

bool platform_command_is_enabled(platform_viewer_action action,
                                 const platform_command_state *state);

#endif
