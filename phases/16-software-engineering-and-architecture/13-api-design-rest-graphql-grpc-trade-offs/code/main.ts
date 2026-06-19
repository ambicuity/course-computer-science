// API Design — REST, GraphQL, gRPC Trade-offs
// The same User domain modeled three ways: REST, GraphQL, gRPC
// No external deps needed — these are the patterns, not a running server.

// ============================================================
// Shared Domain Types
// ============================================================

type UserId = number;

interface User {
  id: UserId;
  name: string;
  email: string;
  status: "ACTIVE" | "SUSPENDED" | "DEACTIVATED";
  createdAt: string;
  updatedAt: string;
}

interface CreateUserInput {
  name: string;
  email: string;
}

interface UpdateUserInput {
  name?: string;
  email?: string;
  status?: User["status"];
}

interface PaginatedResult<T> {
  data: T[];
  nextCursor: string | null;
  totalCount: number;
}

class UserStore {
  private users: Map<UserId, User> = new Map();
  private nextId: UserId = 1;

  create(input: CreateUserInput): User {
    const id = this.nextId++;
    const now = new Date().toISOString();
    const user: User = {
      id,
      name: input.name,
      email: input.email,
      status: "ACTIVE",
      createdAt: now,
      updatedAt: now,
    };
    this.users.set(id, user);
    return user;
  }

  get(id: UserId): User | undefined {
    return this.users.get(id);
  }

  list(cursor: string | null, pageSize: number, statusFilter?: User["status"]): PaginatedResult<User> {
    let all = Array.from(this.users.values());
    if (statusFilter) {
      all = all.filter((u) => u.status === statusFilter);
    }
    const startIndex = cursor ? parseInt(atob(cursor), 10) : 0;
    const page = all.slice(startIndex, startIndex + pageSize);
    const nextIndex = startIndex + pageSize;
    const nextCursor = nextIndex < all.length ? btoa(String(nextIndex)) : null;
    return {
      data: page,
      nextCursor,
      totalCount: all.length,
    };
  }

  update(id: UserId, input: UpdateUserInput): User | undefined {
    const existing = this.users.get(id);
    if (!existing) return undefined;
    const updated: User = {
      ...existing,
      ...(input.name !== undefined && { name: input.name }),
      ...(input.email !== undefined && { email: input.email }),
      ...(input.status !== undefined && { status: input.status }),
      updatedAt: new Date().toISOString(),
    };
    this.users.set(id, updated);
    return updated;
  }

  delete(id: UserId): boolean {
    return this.users.delete(id);
  }
}

// ============================================================
// 1. REST — Express-style handlers
// ============================================================

// Simulates Express route handlers. In production, these would be
// wired to an Express Router with app.get('/v1/users', ...), etc.

interface RestRequest {
  params: Record<string, string>;
  query: Record<string, string | undefined>;
  body: unknown;
  headers: Record<string, string | undefined>;
}

interface RestResponse<T = unknown> {
  statusCode: number;
  body: T;
  headers?: Record<string, string>;
}

function restGetUser(store: UserStore, req: RestRequest): RestResponse {
  const id = parseInt(req.params.id, 10);
  if (isNaN(id)) {
    return { statusCode: 400, body: { error: "Invalid user ID" } };
  }
  const user = store.get(id);
  if (!user) {
    return { statusCode: 404, body: { error: "User not found" } };
  }
  // HATEOAS links — clients discover related actions without hard-coding
  return {
    statusCode: 200,
    body: {
      ...user,
      _links: {
        self: { href: `/v1/users/${user.id}` },
        orders: { href: `/v1/users/${user.id}/orders` },
        deactivate: { href: `/v1/users/${user.id}/deactivate`, method: "POST" },
      },
    },
  };
}

function restListUsers(store: UserStore, req: RestRequest): RestResponse {
  const pageSize = Math.min(parseInt(req.query.pageSize ?? "20", 10), 100);
  const cursor = req.query.cursor ?? null;
  const statusFilter = req.query.status as User["status"] | undefined;
  const result = store.list(cursor, pageSize, statusFilter);
  const links: string[] = [];
  if (result.nextCursor) {
    links.push(`</v1/users?cursor=${result.nextCursor}&pageSize=${pageSize}>; rel="next"`);
  }
  return {
    statusCode: 200,
    body: {
      data: result.data,
      pagination: {
        nextCursor: result.nextCursor,
        totalCount: result.totalCount,
      },
    },
    headers: links.length ? { Link: links.join(", ") } : undefined,
  };
}

function restCreateUser(store: UserStore, req: RestRequest): RestResponse {
  const input = req.body as CreateUserInput;
  if (!input?.name || !input?.email) {
    return { statusCode: 422, body: { error: "name and email are required" } };
  }
  const user = store.create(input);
  return {
    statusCode: 201,
    body: user,
    headers: { Location: `/v1/users/${user.id}` },
  };
}

function restUpdateUser(store: UserStore, req: RestRequest): RestResponse {
  const id = parseInt(req.params.id, 10);
  if (isNaN(id)) {
    return { statusCode: 400, body: { error: "Invalid user ID" } };
  }
  const input = req.body as UpdateUserInput;
  const user = store.update(id, input);
  if (!user) {
    return { statusCode: 404, body: { error: "User not found" } };
  }
  return { statusCode: 200, body: user };
}

function restDeleteUser(store: UserStore, req: RestRequest): RestResponse {
  const id = parseInt(req.params.id, 10);
  if (isNaN(id)) {
    return { statusCode: 400, body: { error: "Invalid user ID" } };
  }
  // Idempotent: DELETE on a non-existent resource is 204
  const existed = store.delete(id);
  return { statusCode: 204, body: existed ? { deleted: true } : null };
}

// ============================================================
// 2. GraphQL — Schema + Resolvers
// ============================================================

// Simulates GraphQL type definitions and resolver functions.
// In production, these would be passed to Apollo Server or graphql-js.

const graphqlSchema = `
  type User {
    id: ID!
    name: String!
    email: String!
    status: UserStatus!
    createdAt: String!
    updatedAt: String!
    orders: [Order!]!
  }

  enum UserStatus {
    ACTIVE
    SUSPENDED
    DEACTIVATED
  }

  type Order {
    id: ID!
    total: Float!
    items: [Item!]!
  }

  type Item {
    id: ID!
    name: String!
    price: Float!
    quantity: Int!
  }

  input CreateUserInput {
    name: String!
    email: String!
  }

  input UpdateUserInput {
    name: String
    email: String
    status: UserStatus
  }

  input UserFilter {
    status: UserStatus
    cursor: String
    pageSize: Int = 20
  }

  type UserConnection {
    edges: [UserEdge!]!
    pageInfo: PageInfo!
    totalCount: Int!
  }

  type UserEdge {
    node: User!
    cursor: String!
  }

  type PageInfo {
    hasNextPage: Boolean!
    endCursor: String
  }

  type Query {
    user(id: ID!): User
    users(filter: UserFilter): UserConnection!
  }

  type Mutation {
    createUser(input: CreateUserInput!): User!
    updateUser(id: ID!, input: UpdateUserInput!): User!
    deleteUser(id: ID!): Boolean!
  }

  type Subscription {
    userUpdated(id: ID!): User!
    userStatusChanged: User!
  }
`;

interface GqlContext {
  store: UserStore;
}

// Resolvers — each field maps to a function
const graphqlResolvers = {
  Query: {
    user: (_root: unknown, args: { id: string }, ctx: GqlContext): User | undefined => {
      return ctx.store.get(parseInt(args.id, 10));
    },
    users: (_root: unknown, args: { filter?: { status?: User["status"]; cursor?: string; pageSize?: number } }, ctx: GqlContext) => {
      const filter = args.filter ?? {};
      const result = ctx.store.list(
        filter.cursor ?? null,
        Math.min(filter.pageSize ?? 20, 100),
        filter.status
      );
      return {
        edges: result.data.map((user) => ({
          node: user,
          cursor: btoa(String(user.id)),
        })),
        pageInfo: {
          hasNextPage: result.nextCursor !== null,
          endCursor: result.nextCursor,
        },
        totalCount: result.totalCount,
      };
    },
  },
  Mutation: {
    createUser: (_root: unknown, args: { input: CreateUserInput }, ctx: GqlContext): User => {
      return ctx.store.create(args.input);
    },
    updateUser: (_root: unknown, args: { id: string; input: UpdateUserInput }, ctx: GqlContext): User | undefined => {
      const user = ctx.store.update(parseInt(args.id, 10), args.input);
      if (!user) throw new Error(`User ${args.id} not found`);
      return user;
    },
    deleteUser: (_root: unknown, args: { id: string }, ctx: GqlContext): boolean => {
      return ctx.store.delete(parseInt(args.id, 10));
    },
  },
  // Nested field resolver: User.orders fetches orders for a specific user
  User: {
    orders: (parent: User): { id: string; total: number; items: never[] }[] => {
      // In production, this would call an OrderService
      console.log(`[GraphQL] Resolving orders for user ${parent.id}`);
      return [];
    },
  },
  // Subscription resolvers would use PubSub / WebSocket in production
  Subscription: {
    userUpdated: {
      subscribe: (_root: unknown, args: { id: string }, _ctx: GqlContext) => {
        console.log(`[GraphQL] Subscribing to updates for user ${args.id}`);
        // In production: return ctx.pubsub.asyncIterator("USER_UPDATED", { id: args.id })
        return { [Symbol.asyncIterator]: () => ({ next: async () => ({ done: true, value: undefined }) }) };
      },
    },
    userStatusChanged: {
      subscribe: (_root: unknown, _args: unknown, _ctx: GqlContext) => {
        console.log("[GraphQL] Subscribing to any user status change");
        return { [Symbol.asyncIterator]: () => ({ next: async () => ({ done: true, value: undefined }) }) };
      },
    },
  },
};

// ============================================================
// 3. gRPC — Service stubs (TypeScript)
// ============================================================

// Simulates gRPC-generated client/server stubs.
// In production, protoc would generate these from main.proto.

// --- Generated message types (from .proto) ---

interface GetUserRequest { id: number; }
interface ListUsersRequest { pageSize: number; pageToken: string; statusFilter: number; }
interface ListUsersResponse { users: User[]; nextPageToken: string; totalCount: number; }
interface CreateUserRequestGrpc { name: string; email: string; }
interface UpdateUserRequestGrpc { id: number; name: string; email: string; status?: number; }
interface DeleteUserRequestGrpc { id: number; }
interface DeleteUserResponse { success: boolean; }
interface StreamUsersRequest { statusFilter: number; }

// --- Unary RPC handlers (server-side) ---

interface UserServiceServer {
  getUser(request: GetUserRequest): Promise<User>;
  listUsers(request: ListUsersRequest): Promise<ListUsersResponse>;
  createUser(request: CreateUserRequestGrpc): Promise<User>;
  updateUser(request: UpdateUserRequestGrpc): Promise<User>;
  deleteUser(request: DeleteUserRequestGrpc): Promise<DeleteUserResponse>;
  streamUsers(request: StreamUsersRequest): AsyncIterable<User>;
}

function createUserServiceHandler(store: UserStore): UserServiceServer {
  const statusMap: Record<number, User["status"]> = {
    0: undefined as unknown as User["status"], // UNSPECIFIED = no filter
    1: "ACTIVE",
    2: "SUSPENDED",
    3: "DEACTIVATED",
  };

  return {
    async getUser(request: GetUserRequest): Promise<User> {
      const user = store.get(request.id);
      if (!user) {
        // In production gRPC, this would be a NOT_FOUND status code
        throw { code: 5, message: `User ${request.id} not found` }; // gRPC NOT_FOUND
      }
      return user;
    },

    async listUsers(request: ListUsersRequest): Promise<ListUsersResponse> {
      const statusFilter = statusMap[request.statusFilter];
      const result = store.list(
        request.pageToken || null,
        Math.min(request.pageSize || 20, 100),
        statusFilter
      );
      return {
        users: result.data,
        nextPageToken: result.nextCursor ?? "",
        totalCount: result.totalCount,
      };
    },

    async createUser(request: CreateUserRequestGrpc): Promise<User> {
      if (!request.name || !request.email) {
        // gRPC INVALID_ARGUMENT
        throw { code: 3, message: "name and email are required" };
      }
      return store.create({ name: request.name, email: request.email });
    },

    async updateUser(request: UpdateUserRequestGrpc): Promise<User> {
      const user = store.update(request.id, {
        ...(request.name && { name: request.name }),
        ...(request.email && { email: request.email }),
        ...(request.status !== undefined && { status: statusMap[request.status] }),
      });
      if (!user) {
        throw { code: 5, message: `User ${request.id} not found` };
      }
      return user;
    },

    async deleteUser(request: DeleteUserRequestGrpc): Promise<DeleteUserResponse> {
      const existed = store.delete(request.id);
      return { success: true };
    },

    // Server streaming: returns an AsyncIterable that yields users as they change
    async *streamUsers(request: StreamUsersRequest): AsyncIterable<User> {
      const statusFilter = statusMap[request.statusFilter];
      // In production, this would watch a changelog or event stream
      console.log(`[gRPC] Streaming users with status filter: ${statusFilter}`);
      const current = store.list(null, 1000, statusFilter);
      for (const user of current.data) {
        yield user;
      }
      // Keep the stream open for new events (simulated: close immediately)
    },
  };
}

// --- Client-side stub ---

interface UserServiceClient {
  getUser(request: GetUserRequest): Promise<User>;
  listUsers(request: ListUsersRequest): Promise<ListUsersResponse>;
  createUser(request: CreateUserRequestGrpc): Promise<User>;
  updateUser(request: UpdateUserRequestGrpc): Promise<User>;
  deleteUser(request: DeleteUserRequestGrpc): Promise<DeleteUserResponse>;
  streamUsers(request: StreamUsersRequest): AsyncIterable<User>;
}

// ============================================================
// Demonstration: Same data, three paradigms
// ============================================================

function main(): void {
  const store = new UserStore();

  // Seed data
  const alice = store.create({ name: "Alice", email: "alice@example.com" });
  store.create({ name: "Bob", email: "bob@example.com" });
  store.create({ name: "Carol", email: "carol@example.com" });

  console.log("=== REST Demo ===");
  const restGetResp = restGetUser(store, { params: { id: "1" }, query: {}, body: {}, headers: {} });
  console.log("GET /v1/users/1 =>", JSON.stringify(restGetResp, null, 2));

  const restListResp = restListUsers(store, { params: {}, query: { pageSize: "2" }, body: {}, headers: {} });
  console.log("GET /v1/users?pageSize=2 =>", JSON.stringify(restListResp, null, 2));

  const restCreateResp = restCreateUser(store, { params: {}, query: {}, body: { name: "Dave", email: "dave@example.com" }, headers: {} });
  console.log("POST /v1/users (Dave) =>", JSON.stringify(restCreateResp, null, 2));

  const restDeleteResp = restDeleteUser(store, { params: { id: "1" }, query: {}, body: {}, headers: {} });
  console.log("DELETE /v1/users/1 =>", JSON.stringify(restDeleteResp, null, 2));

  console.log("\n=== GraphQL Demo ===");
  const ctx: GqlContext = { store };
  const gqlUser = graphqlResolvers.Query.user({}, { id: String(alice.id) }, ctx);
  console.log("query { user(id: 1) } =>", JSON.stringify(gqlUser, null, 2));

  const gqlUsers = graphqlResolvers.Query.users({}, { filter: { pageSize: 2 } }, ctx);
  console.log("query { users(filter: { pageSize: 2 }) } =>", JSON.stringify(gqlUsers, null, 2));

  const gqlCreated = graphqlResolvers.Mutation.createUser({}, { input: { name: "Eve", email: "eve@example.com" } }, ctx);
  console.log("mutation { createUser(input: { name: \"Eve\" }) } =>", JSON.stringify(gqlCreated, null, 2));

  console.log("\n=== gRPC Demo ===");
  const grpcServer = createUserServiceHandler(store);
  grpcServer.getUser({ id: 2 }).then((user) => {
    console.log("GetUser(id=2) =>", JSON.stringify(user, null, 2));
  });

  grpcServer.listUsers({ pageSize: 10, pageToken: "", statusFilter: 1 }).then((resp) => {
    console.log("ListUsers(status=ACTIVE) =>", JSON.stringify(resp, null, 2));
  });

  grpcServer.createUser({ name: "Frank", email: "frank@example.com" }).then((user) => {
    console.log("CreateUser(Frank) =>", JSON.stringify(user, null, 2));
  });

  // Backward compatibility demo
  console.log("\n=== Backward Compatibility ===");
  console.log("Adding a field to User (e.g., 'phone') is SAFE — old clients ignore it.");
  console.log("Removing a field requires: 1) Mark deprecated, 2) Communicate, 3) Reserve field number, 4) Never reuse.");
  console.log("In protobuf: reserved 7; reserved 'nickname'; — prevents accidental reuse.");

  console.log("\n=== Versioning Strategy ===");
  console.log("REST:      /v1/users, /v2/users          (URL path — explicit, cacheable)");
  console.log("GraphQL:  Schema evolution (add fields)  (No URL versioning — add, deprecate, never remove)");
  console.log("gRPC:     Proto field numbers are forever  (Add fields with new numbers, reserve old ones)");
}

main();