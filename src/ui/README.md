# UI state

UI state values are owner-thread-only snapshots. The index progress adapter
converts generation-tagged byte progress into a small loading indicator model;
rendering a native progress control remains the window layer's responsibility.
