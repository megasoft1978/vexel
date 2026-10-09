# UI lane

The first editor chrome demo is composed from a retained widget tree and uses
logical pixels for layout. The current golden fixture checks deterministic
1280x800 screenshots for dark and light themes. Regenerate only after visual
review with `python3 tests/ui/verify_chrome.py --update`.
