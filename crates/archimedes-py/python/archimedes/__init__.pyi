"""Type stubs for Archimedes Python bindings."""

from typing import Any, Awaitable, Callable, Optional

class Identity:
    """Caller identity from authentication."""
    
    @property
    def subject(self) -> str:
        """Subject identifier (e.g., user ID)."""
        ...
    
    @property
    def issuer(self) -> Optional[str]:
        """Token issuer."""
        ...
    
    @property
    def audience(self) -> Optional[str]:
        """Token audience."""
        ...
    
    @property
    def roles(self) -> list[str]:
        """List of roles."""
        ...
    
    @property
    def permissions(self) -> list[str]:
        """List of permissions."""
        ...
    
    @property
    def metadata(self) -> dict[str, Any]:
        """Additional identity claims."""
        ...
    
    def has_role(self, role: str) -> bool:
        """Check if identity has a specific role."""
        ...
    
    def has_permission(self, permission: str) -> bool:
        """Check if identity has a specific permission."""
        ...
    
    def is_expired(self) -> bool:
        """Check if the identity token is expired."""
        ...


class RequestContext:
    """Request context passed to handlers."""
    
    @property
    def operation_id(self) -> str:
        """The operation ID being handled."""
        ...
    
    @property
    def path_params(self) -> dict[str, str]:
        """Path parameters extracted from the URL."""
        ...
    
    @property
    def query_params(self) -> dict[str, str]:
        """Query parameters from the URL."""
        ...
    
    @property
    def headers(self) -> dict[str, str]:
        """Request headers."""
        ...
    
    @property
    def body(self) -> Any:
        """Parsed request body."""
        ...
    
    @property
    def identity(self) -> Optional[Identity]:
        """Caller identity if authenticated."""
        ...
    
    @property
    def trace_id(self) -> Optional[str]:
        """OpenTelemetry trace ID."""
        ...
    
    @property
    def span_id(self) -> Optional[str]:
        """OpenTelemetry span ID."""
        ...


class Response:
    """HTTP response returned from handlers."""
    
    status: int
    body: Any
    headers: dict[str, str]
    
    def __init__(
        self,
        status: int = 200,
        body: Any = None,
        headers: Optional[dict[str, str]] = None,
    ) -> None:
        """Create a new response."""
        ...
    
    def set_header(self, name: str, value: str) -> None:
        """Set a response header."""
        ...
    
    def get_header(self, name: str) -> Optional[str]:
        """Get a response header."""
        ...
    
    @staticmethod
    def ok(
        body: Any = None,
        headers: Optional[dict[str, str]] = None,
    ) -> "Response":
        """Create an OK response (200)."""
        ...
    
    @staticmethod
    def created(
        body: Any = None,
        headers: Optional[dict[str, str]] = None,
    ) -> "Response":
        """Create a Created response (201)."""
        ...
    
    @staticmethod
    def no_content() -> "Response":
        """Create a No Content response (204)."""
        ...
    
    @staticmethod
    def bad_request(message: Optional[str] = None) -> "Response":
        """Create a Bad Request response (400)."""
        ...
    
    @staticmethod
    def unauthorized(message: Optional[str] = None) -> "Response":
        """Create an Unauthorized response (401)."""
        ...
    
    @staticmethod
    def forbidden(message: Optional[str] = None) -> "Response":
        """Create a Forbidden response (403)."""
        ...
    
    @staticmethod
    def not_found(message: Optional[str] = None) -> "Response":
        """Create a Not Found response (404)."""
        ...
    
    @staticmethod
    def internal_error(message: Optional[str] = None) -> "Response":
        """Create an Internal Server Error response (500)."""
        ...
    
    @staticmethod
    def json(body: Any, status: int = 200) -> "Response":
        """Create a JSON response with Content-Type header."""
        ...


class Config:
    """Configuration for an Archimedes application."""
    
    contract_path: str
    listen_port: int
    listen_addr: str
    enable_telemetry: bool
    log_level: str
    service_name: str
    opa_bundle_url: Optional[str]
    
    def __init__(
        self,
        contract_path: str,
        listen_port: int = 8080,
        listen_addr: str = "0.0.0.0",
        enable_telemetry: bool = False,
        log_level: str = "info",
        service_name: str = "archimedes-python",
        opa_bundle_url: Optional[str] = None,
    ) -> None:
        """Create application configuration."""
        ...
    
    @staticmethod
    def from_file(path: str) -> "Config":
        """Load configuration from a YAML or JSON file."""
        ...
    
    @staticmethod
    def from_env() -> "Config":
        """Load configuration from environment variables."""
        ...
    
    def bind_address(self) -> str:
        """Get the full bind address (addr:port)."""
        ...


HandlerFunc = Callable[[RequestContext], Response | dict[str, Any] | Awaitable[Response | dict[str, Any]]]


class App:
    """Archimedes application instance."""
    
    def __init__(self, config: Config) -> None:
        """Create a new Archimedes application."""
        ...
    
    def handler(self, operation_id: str) -> Callable[[HandlerFunc], HandlerFunc]:
        """Decorator to register a handler for an operation."""
        ...
    
    def register_handler(self, operation_id: str, handler: HandlerFunc) -> None:
        """Register a handler function directly."""
        ...
    
    def run(self) -> None:
        """Run the application (blocking)."""
        ...
    
    async def run_async(self) -> None:
        """Run the application asynchronously."""
        ...


class ArchimedesError(Exception):
    """Base exception for Archimedes errors."""
    ...
