/**
 * TypeScript Native Example - User CRUD Service
 * 
 * This example demonstrates using Archimedes native bindings (@archimedes/node)
 * instead of Express or other Node.js frameworks.
 * 
 * Features:
 * - Contract-first API definition
 * - Built-in middleware (request ID, tracing, identity, authorization)
 * - Automatic request/response validation
 * - Type-safe handlers
 */

import {
  Archimedes,
  Request,
  Response,
  Config,
  RequestContext,
} from '@archimedes/node';

// =============================================================================
// Types
// =============================================================================

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

interface UsersResponse {
  users: User[];
  total: number;
}

interface HealthResponse {
  status: string;
  service: string;
  timestamp: string;
}

// =============================================================================
// In-Memory Database
// =============================================================================

const usersDb = new Map<string, User>([
  ['1', {
    id: '1',
    name: 'Alice Smith',
    email: 'alice@example.com',
    created_at: '2026-01-01T00:00:00Z',
  }],
  ['2', {
    id: '2',
    name: 'Bob Johnson',
    email: 'bob@example.com',
    created_at: '2026-01-02T00:00:00Z',
  }],
]);

let nextId = 3;

// =============================================================================
// Application Setup
// =============================================================================

const config = new Config({
  contractPath: '../contract.json',
  listenPort: 8004,
  serviceName: 'typescript-native-example',
  enableValidation: true,
  enableTracing: true,
});

const app = new Archimedes(config);

// =============================================================================
// Health Check Handler
// =============================================================================

app.operation('healthCheck', async (request: Request): Promise<Response> => {
  const response: HealthResponse = {
    status: 'healthy',
    service: 'typescript-native-example',
    timestamp: new Date().toISOString(),
  };
  return Response.json(response);
});

// =============================================================================
// User CRUD Handlers
// =============================================================================

/**
 * List all users
 * GET /users
 */
app.operation('listUsers', async (request: Request): Promise<Response> => {
  const users = Array.from(usersDb.values());
  
  const response: UsersResponse = {
    users,
    total: users.length,
  };
  
  return Response.json(response);
});

/**
 * Get a specific user by ID
 * GET /users/:userId
 */
app.operation('getUser', async (request: Request): Promise<Response> => {
  const userId = request.pathParams.userId;
  
  if (!userId) {
    return Response.badRequest({
      code: 'MISSING_USER_ID',
      message: 'User ID is required',
      request_id: request.requestId,
    });
  }
  
  const user = usersDb.get(userId);
  
  if (!user) {
    return Response.notFound({
      code: 'USER_NOT_FOUND',
      message: `User with ID ${userId} not found`,
      request_id: request.requestId,
    });
  }
  
  return Response.json(user);
});

/**
 * Create a new user
 * POST /users
 */
app.operation('createUser', async (request: Request): Promise<Response> => {
  const body = request.json<CreateUserRequest>();
  
  // Validation is handled by Archimedes middleware via contract
  // But we can add business logic validation here
  if (!body.name || !body.email) {
    return Response.badRequest({
      code: 'INVALID_REQUEST',
      message: 'Name and email are required',
      request_id: request.requestId,
    });
  }
  
  // Check for duplicate email
  for (const user of usersDb.values()) {
    if (user.email === body.email) {
      return Response.status(409).json({
        code: 'DUPLICATE_EMAIL',
        message: `User with email ${body.email} already exists`,
        request_id: request.requestId,
      });
    }
  }
  
  const newUser: User = {
    id: String(nextId++),
    name: body.name,
    email: body.email,
    created_at: new Date().toISOString(),
  };
  
  usersDb.set(newUser.id, newUser);
  
  return Response.created(newUser);
});

/**
 * Update an existing user
 * PUT /users/:userId
 */
app.operation('updateUser', async (request: Request): Promise<Response> => {
  const userId = request.pathParams.userId;
  
  if (!userId) {
    return Response.badRequest({
      code: 'MISSING_USER_ID',
      message: 'User ID is required',
      request_id: request.requestId,
    });
  }
  
  const user = usersDb.get(userId);
  
  if (!user) {
    return Response.notFound({
      code: 'USER_NOT_FOUND',
      message: `User with ID ${userId} not found`,
      request_id: request.requestId,
    });
  }
  
  const body = request.json<UpdateUserRequest>();
  
  // Apply partial updates
  if (body.name !== undefined) {
    user.name = body.name;
  }
  if (body.email !== undefined) {
    // Check for duplicate email
    for (const [id, existingUser] of usersDb.entries()) {
      if (id !== userId && existingUser.email === body.email) {
        return Response.status(409).json({
          code: 'DUPLICATE_EMAIL',
          message: `User with email ${body.email} already exists`,
          request_id: request.requestId,
        });
      }
    }
    user.email = body.email;
  }
  
  usersDb.set(userId, user);
  
  return Response.json(user);
});

/**
 * Delete a user
 * DELETE /users/:userId
 */
app.operation('deleteUser', async (request: Request): Promise<Response> => {
  const userId = request.pathParams.userId;
  
  if (!userId) {
    return Response.badRequest({
      code: 'MISSING_USER_ID',
      message: 'User ID is required',
      request_id: request.requestId,
    });
  }
  
  if (!usersDb.has(userId)) {
    return Response.notFound({
      code: 'USER_NOT_FOUND',
      message: `User with ID ${userId} not found`,
      request_id: request.requestId,
    });
  }
  
  usersDb.delete(userId);
  
  return Response.noContent();
});

// =============================================================================
// Start Server
// =============================================================================

async function main(): Promise<void> {
  console.log('Starting TypeScript Native Example Service...');
  console.log(`Contract: ${config.contractPath}`);
  console.log(`Port: ${config.listenPort}`);
  
  try {
    await app.listen(config.listenPort);
  } catch (error) {
    console.error('Failed to start server:', error);
    process.exit(1);
  }
}

main();
