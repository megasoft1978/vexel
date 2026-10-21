-- Helper for scripts/e2e_ui.sh. macOS built-ins only (System Events).
-- usage: osascript scripts/e2e_ui.applescript <process> <command> [args...]
-- commands:
--   focus                       bring the process frontmost
--   has <text>                  "yes"/"no": any UI element whose name, title,
--                               description or value contains <text>
--   click <text>                press/click the first element matching <text>
--   setfield <text> <value>     set the value of the first element matching <text>
--   key <char> [mods]           keystroke; mods = comma list of command,shift,option,control
--   code <keycode> [mods]       key code with the same mods
--   type <text>                 type text
--   windows                     number of windows
-- Prints "ok"/"yes"/"no"/a number on success; "err: ..." and exit 1 on failure.
on run argv
	set procName to item 1 of argv
	set cmd to item 2 of argv
	try
		tell application "System Events"
			if not (exists process procName) then error "process not running"
			tell process procName
				if cmd is "focus" then
					set frontmost to true
					return "ok"
				else if cmd is "windows" then
					return (count of windows) as text
				else if cmd is "has" then
					return my findMatch(procName, item 3 of argv, false)
				else if cmd is "click" then
					return my findMatch(procName, item 3 of argv, true)
				else if cmd is "setfield" then
					set els to entire contents of window 1
					repeat with e in els
						if my matches(e, item 3 of argv) then
							try
								set value of e to item 4 of argv
								return "ok"
							end try
						end if
					end repeat
					error "no settable element: " & (item 3 of argv)
				else if cmd is "key" then
					my press(item 3 of argv, argv, 4, false)
					return "ok"
				else if cmd is "code" then
					my press(item 3 of argv, argv, 4, true)
					return "ok"
				else if cmd is "type" then
					keystroke (item 3 of argv)
					return "ok"
				end if
			end tell
		end tell
		error "unknown command " & cmd
	on error msg
		return "err: " & msg
	end try
end run

on matches(e, needle)
	tell application "System Events"
		repeat with attr in {"AXTitle", "AXDescription", "AXValue", "AXHelp"}
			try
				set v to value of attribute (contents of attr) of e
				if v is not missing value then
					if (v as text) contains needle then return true
				end if
			end try
		end repeat
	end tell
	return false
end matches

on findMatch(procName, needle, doClick)
	tell application "System Events"
		tell process procName
			-- Sheets/panels first (save panel), then main window.
			repeat with w in windows
				set els to entire contents of w
				repeat with e in els
					if my matches(e, needle) then
						if not doClick then return "yes"
						try
							perform action "AXPress" of e
						on error
							set {x, y} to position of e
							set {sx, sy} to size of e
							click at {x + sx / 2, y + sy / 2}
						end try
						return "ok"
					end if
				end repeat
			end repeat
		end tell
	end tell
	if doClick then return "err: not found: " & needle
	return "no"
end findMatch

on press(arg, argv, modIndex, isCode)
	set mods to {}
	if (count of argv) ≥ modIndex then
		set spec to item modIndex of argv
		if spec contains "command" then set end of mods to command down
		if spec contains "shift" then set end of mods to shift down
		if spec contains "option" then set end of mods to option down
		if spec contains "control" then set end of mods to control down
	end if
	tell application "System Events"
		if isCode then
			if (count of mods) = 0 then
				key code (arg as integer)
			else
				key code (arg as integer) using mods
			end if
		else
			if (count of mods) = 0 then
				keystroke arg
			else
				keystroke arg using mods
			end if
		end if
	end tell
end press
