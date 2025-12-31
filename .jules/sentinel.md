## 2025-02-18 - Logging in Libraries
**Vulnerability:** The client library was using `print()` to output error responses. This is bad practice (pollutes stdout) and can leak sensitive information if not handled correctly.
**Learning:** Libraries should always use the `logging` module. This allows the application using the library to control log levels and handlers, and prevents uncontrolled leakage of data to stdout.
**Prevention:** Use `logging.getLogger(__name__)` in library modules.
