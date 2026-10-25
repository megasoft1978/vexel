class HelperError(Exception):
    """A request failed; `code` is a stable machine-readable reason."""

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message
