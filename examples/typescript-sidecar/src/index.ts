/**
 * TypeScript Example Service with Archimedes Sidecar
 *
 * This service demonstrates how to build a TypeScript/Express service that works
 * with the Archimedes sidecar for contract validation, authorization, and observability.
 */

import express, { Request, Response, NextFunction } from "express";
import { v4 as uuidv4 } from "uuid";

// =============================================================================
// Types
// =============================================================================

interface CallerIdentity {
  type: "spiffe" | "user" | "api_key" | "anonymous";
  id?: string;
  // SPIFFE-specific
  trust_domain?: string;
  path?: string;
  // User-specific
  user_id?: string;
  roles?: string[];
  // API Key-specific
  key_id?: string;
}

interface User {
  id: string;
  name: string;
  email: string;
  created_at: string;
}

interface CreateUserRequest {
  name: string;
  email: string;
}

interface UpdateUserRequest {
  name?: string;
  email?: string;
}

interface HealthResponse {
  status: string;
  service: string;
  timestamp: string;
}

interface UsersResponse {
  users: User[];
  total: number;
}

interface ErrorResponse {
  code: string;
  message: string;
  request_id?: string;
}

interface RequestContext {
  requestId: string;
  caller: CallerIdentity | null;
  operationId: string | null;
}

// =============================================================================
// In-Memory Database
// =============================================================================

const usersDb = new Map<string, User>([
  [
    "1",
    {
      id: "1",
      name: "Alice Smith",
      email: "alice@example.com",
      created_at: "2026-01-01T00:00:00Z",
    },
  ],
  [
    "2",
    {
      id: "2",
      name: "Bob Johnson",
      email: "bob@example.com",
      created_at: "2026-01-02T00:00:00Z",
    },
  ],
]);

// =============================================================================
// Helper Functions
// =============================================================================

function parseCallerIdentity(
  headerValue: string | undefined
): CallerIdentity | null {
  if (!headerValue) {
    return null;
  }
  try {
    return JSON.parse(headerValue) as CallerIdentity;
  } catch (e) {
    console.warn("Failed to parse caller identity:", e);
    return null;
  }
}

function getRequestContext(req: Request): RequestContext {
  const requestId = (req.headers["x-request-id"] as string) || uuidv4();
  const caller = parseCallerIdentity(
    req.headers["x-caller-identity"] as string
  );
  const operationId = (req.headers["x-operation-id"] as string) || null;

  return {
    requestId,
    caller,
    operationId,
  };
}

function sendError(
  res: Response,
  status: number,
  code: string,
  message: string,
  requestId?: string
): void {
  const error: ErrorResponse = { code, message };
  if (requestId) {
    error.request_id = requestId;
  }
  res.status(status).json(error);
}

// =============================================================================
// Express App
// =============================================================================

const app = express();
app.use(express.json());

// Request logging middleware
app.use((req: Request, res: Response, next: NextFunction) => {
  const ctx = getRequestContext(req);
  console.log(`[${ctx.requestId}] ${req.method} ${req.path}`);
  next();
});

// =============================================================================
// Health Endpoint
// =============================================================================

app.get("/health", (_req: Request, res: Response) => {
  const response: HealthResponse = {
    status: "healthy",
    service: "example-typescript",
    timestamp: new Date().toISOString(),
  };
  res.json(response);
});

// =============================================================================
// User Endpoints
// =============================================================================

// List users
app.get("/users", (req: Request, res: Response) => {
  const ctx = getRequestContext(req);
  console.log(`[${ctx.requestId}] Listing users, caller:`, ctx.caller);

  const users = Array.from(usersDb.values());
  const response: UsersResponse = {
    users,
    total: users.length,
  };
  res.json(response);
});

// Get user by ID
app.get("/users/:userId", (req: Request, res: Response) => {
  const ctx = getRequestContext(req);
  const { userId } = req.params;
  console.log(`[${ctx.requestId}] Getting user ${userId}, caller:`, ctx.caller);

  const user = usersDb.get(userId);
  if (!user) {
    return sendError(
      res,
      404,
      "USER_NOT_FOUND",
      `User with ID '${userId}' not found`,
      ctx.requestId
    );
  }
  res.json(user);
});

// Create user
app.post("/users", (req: Request, res: Response) => {
  const ctx = getRequestContext(req);
  console.log(`[${ctx.requestId}] Creating user, caller:`, ctx.caller);

  const body = req.body as CreateUserRequest;

  // Validate request
  if (!body.name || !body.email) {
    return sendError(
      res,
      400,
      "INVALID_REQUEST",
      "Name and email are required",
      ctx.requestId
    );
  }

  // Check for duplicate email
  for (const user of usersDb.values()) {
    if (user.email === body.email) {
      return sendError(
        res,
        400,
        "EMAIL_EXISTS",
        `User with email '${body.email}' already exists`,
        ctx.requestId
      );
    }
  }

  const user: User = {
    id: uuidv4(),
    name: body.name,
    email: body.email,
    created_at: new Date().toISOString(),
  };
  usersDb.set(user.id, user);

  console.log(`[${ctx.requestId}] Created user ${user.id}`);
  res.status(201).json(user);
});

// Update user
app.put("/users/:userId", (req: Request, res: Response) => {
  const ctx = getRequestContext(req);
  const { userId } = req.params;
  console.log(
    `[${ctx.requestId}] Updating user ${userId}, caller:`,
    ctx.caller
  );

  const user = usersDb.get(userId);
  if (!user) {
    return sendError(
      res,
      404,
      "USER_NOT_FOUND",
      `User with ID '${userId}' not found`,
      ctx.requestId
    );
  }

  const body = req.body as UpdateUserRequest;
  const updatedUser: User = {
    ...user,
    name: body.name ?? user.name,
    email: body.email ?? user.email,
  };
  usersDb.set(userId, updatedUser);

  console.log(`[${ctx.requestId}] Updated user ${userId}`);
  res.json(updatedUser);
});

// Delete user
app.delete("/users/:userId", (req: Request, res: Response) => {
  const ctx = getRequestContext(req);
  const { userId } = req.params;
  console.log(
    `[${ctx.requestId}] Deleting user ${userId}, caller:`,
    ctx.caller
  );

  if (!usersDb.has(userId)) {
    return sendError(
      res,
      404,
      "USER_NOT_FOUND",
      `User with ID '${userId}' not found`,
      ctx.requestId
    );
  }

  usersDb.delete(userId);
  console.log(`[${ctx.requestId}] Deleted user ${userId}`);
  res.status(204).send();
});

// =============================================================================
// Server
// =============================================================================

const port = parseInt(process.env.PORT || "3000", 10);
const host = process.env.HOST || "0.0.0.0";

app.listen(port, host, () => {
  console.log(`TypeScript example service listening on ${host}:${port}`);
});
