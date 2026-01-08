"""
Archimedes Python Bindings

Native Python bindings for the Archimedes HTTP server framework.

Example:
    from archimedes import App, Config, Response

    config = Config(contract_path="contract.json")
    app = App(config)

    @app.handler("getUser")
    def get_user(ctx):
        return Response.ok({"id": ctx.path_params["userId"]})

    app.run()
"""

# Re-export all public symbols from the native extension
from archimedes._archimedes import (
    App,
    Config,
    Response,
    RequestContext,
    Identity,
    ArchimedesError,
)

__all__ = [
    "App",
    "Config",
    "Response",
    "RequestContext",
    "Identity",
    "ArchimedesError",
]

__version__ = "0.1.0"
