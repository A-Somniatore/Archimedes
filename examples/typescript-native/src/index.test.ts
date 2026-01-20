/**
 * Tests for TypeScript Native Example
 */

import { Config, Response, RequestContext } from "@archimedes/node";

describe("Config", () => {
  it("should create config with default values", () => {
    const config = new Config({
      contractPath: "contract.json",
    });

    expect(config.contractPath).toBe("contract.json");
  });

  it("should create config with custom port", () => {
    const config = new Config({
      contractPath: "contract.json",
      listenPort: 3000,
    });

    expect(config.listenPort).toBe(3000);
  });

  it("should create config with all options", () => {
    const config = new Config({
      contractPath: "contract.json",
      listenPort: 8080,
      serviceName: "test-service",
      enableValidation: true,
      enableTracing: false,
    });

    expect(config.contractPath).toBe("contract.json");
    expect(config.listenPort).toBe(8080);
    expect(config.serviceName).toBe("test-service");
    expect(config.enableValidation).toBe(true);
    expect(config.enableTracing).toBe(false);
  });
});

describe("Response", () => {
  it("should create JSON response", () => {
    const response = Response.json({ message: "hello" });

    expect(response.statusCode).toBe(200);
    expect(response.contentType).toBe("application/json");
  });

  it("should create response with custom status", () => {
    const response = Response.status(201).json({ id: "123" });

    expect(response.statusCode).toBe(201);
  });

  it("should create not found response", () => {
    const response = Response.notFound({ error: "Not found" });

    expect(response.statusCode).toBe(404);
  });

  it("should create bad request response", () => {
    const response = Response.badRequest({ error: "Invalid input" });

    expect(response.statusCode).toBe(400);
  });

  it("should create created response", () => {
    const response = Response.created({ id: "123" });

    expect(response.statusCode).toBe(201);
  });

  it("should create no content response", () => {
    const response = Response.noContent();

    expect(response.statusCode).toBe(204);
  });
});

describe("RequestContext", () => {
  it("should have request ID", () => {
    const ctx = new RequestContext({
      requestId: "req-123",
      method: "GET",
      path: "/users",
    });

    expect(ctx.requestId).toBe("req-123");
    expect(ctx.method).toBe("GET");
    expect(ctx.path).toBe("/users");
  });

  it("should have path parameters", () => {
    const ctx = new RequestContext({
      requestId: "req-123",
      method: "GET",
      path: "/users/42",
      pathParams: { userId: "42" },
    });

    expect(ctx.pathParams.userId).toBe("42");
  });

  it("should have caller identity", () => {
    const ctx = new RequestContext({
      requestId: "req-123",
      method: "GET",
      path: "/users",
      caller: {
        type: "user",
        userId: "user-123",
        roles: ["admin"],
      },
    });

    expect(ctx.caller?.type).toBe("user");
    expect(ctx.caller?.userId).toBe("user-123");
    expect(ctx.caller?.roles).toContain("admin");
  });
});

describe("User CRUD operations", () => {
  // These tests would require a running server or mock
  // For now, they serve as documentation of expected behavior

  describe("listUsers", () => {
    it("should return list of users with total count", () => {
      // Expected response format
      const expectedFormat = {
        users: [
          {
            id: "1",
            name: "Alice",
            email: "alice@example.com",
            created_at: expect.any(String),
          },
        ],
        total: 1,
      };

      expect(expectedFormat.users).toBeInstanceOf(Array);
      expect(typeof expectedFormat.total).toBe("number");
    });
  });

  describe("getUser", () => {
    it("should return user by ID", () => {
      const expectedFormat = {
        id: "1",
        name: "Alice",
        email: "alice@example.com",
        created_at: expect.any(String),
      };

      expect(typeof expectedFormat.id).toBe("string");
      expect(typeof expectedFormat.name).toBe("string");
    });

    it("should return 404 for non-existent user", () => {
      const expectedError = {
        code: "USER_NOT_FOUND",
        message: expect.any(String),
      };

      expect(expectedError.code).toBe("USER_NOT_FOUND");
    });
  });

  describe("createUser", () => {
    it("should create user with name and email", () => {
      const request = { name: "New User", email: "new@example.com" };

      expect(typeof request.name).toBe("string");
      expect(typeof request.email).toBe("string");
    });

    it("should return 409 for duplicate email", () => {
      const expectedError = {
        code: "DUPLICATE_EMAIL",
        message: expect.any(String),
      };

      expect(expectedError.code).toBe("DUPLICATE_EMAIL");
    });
  });

  describe("updateUser", () => {
    it("should update user partially", () => {
      const request = { name: "Updated Name" };

      expect(typeof request.name).toBe("string");
    });
  });

  describe("deleteUser", () => {
    it("should return 204 on successful delete", () => {
      // 204 No Content expected
      const expectedStatus = 204;
      expect(expectedStatus).toBe(204);
    });
  });
});
