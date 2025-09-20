# WIND Protocol: Architecture and Dataflow
This document explains the architecture and dataflow of the WIND protocol, a distributed messaging system implemented in Rust using Tokio for asynchronous networking. It covers the main components, their interactions, and how Tokio's async model enables high concurrency and performance.

### 1. Overall Architecture and Protocol

The WIND protocol is a distributed messaging system built on TCP, composed of three primary components that work together:

1.  **Registry (`wind-registry`)**: A central server responsible for service discovery. Publishers and RPC servers register themselves here, and clients query it to find where services are located.
2.  **Servers (`wind-server`)**: These are the data and logic providers. This implementation defines two types:
    *   **`Publisher`**: Implements the publish-subscribe (Pub/Sub) pattern. It broadcasts data updates to multiple connected subscribers.
    *   **`RpcServer`**: Implements the Remote Procedure Call (RPC) pattern. It exposes methods that can be invoked by clients.
3.  **Client (`wind-client`)**: A unified library for interacting with WIND services. It can discover services, subscribe to `Publisher` data streams, and make calls to `RpcServer` methods.

All communication uses a binary protocol defined in protocol.rs, where messages are serialized using `bincode` and framed with a 4-byte length prefix, as implemented in the `MessageCodec`.

---

### 2. Dataflow and Component Interaction

#### Step 1: Service Registration

1.  A `Publisher` (`crates/wind-server/src/publisher.rs`) or `RpcServer` (`crates/wind-server/src/rpc_server.rs`) starts. It binds to a TCP port (e.g., `127.0.0.1:0` to get any available port).
2.  It then connects to the `RegistryServer` (`crates/wind-registry/src/server.rs`) and sends a `MessagePayload::RegisterService` message containing its name, actual address, and type (`Publisher` or `RpcServer`).
3.  The `RegistryServer`'s `handle_message` function processes this request, calling `registry.register_service` (`crates/wind-registry/src/registry.rs`).
4.  The `Registry` stores the service information in a thread-safe `DashMap`, associating it with a Time-to-Live (TTL). It responds with a `ServiceRegistered` message.
5.  The `Publisher` also starts a `start_heartbeat_task` to periodically re-register itself, preventing its entry in the registry from expiring.

#### Step 2: Service Discovery

1.  A `WindClient` is created with the registry's address.
2.  When the client calls `subscribe()` or `call()`, it first needs to find the service. It uses its internal `Subscriber` to send a `MessagePayload::DiscoverServices` message to the registry. This message contains a pattern (e.g., `SENSOR/*/TEMP` or an exact name).
3.  The `RegistryServer` receives this, and its `Registry` uses the `ServicePattern` glob matcher to find all matching, non-expired services.
4.  The registry responds with a `ServicesDiscovered` message containing a list of `ServiceInfo` objects.

#### Step 3: Client-Server Communication

*   **For Pub/Sub:**
    1.  After discovering the `Publisher`, the `WindClient` connects directly to the `Publisher`'s address.
    2.  It sends a `MessagePayload::Subscribe` message.
    3.  The `Publisher`'s `spawn_client_listener` task receives this, creates a `ClientSubscription` to track the client's subscription mode (`Once`, `OnChange`, `Periodic`), and sends back a `SubscribeAck`.
    4.  When an external source calls `publisher.publish()`, the new `WindValue` is sent into a `tokio::sync::broadcast` channel.
    5.  The `start_update_sender` task in the `Publisher` receives the value from the broadcast channel and iterates through all connected clients. It checks if the update should be sent based on each client's `SubscriptionMode` and sends a `Publish` message if needed.

*   **For RPC:**
    1.  After discovering the `RpcServer`, the `WindClient` connects directly to its address.
    2.  It sends a `MessagePayload::RpcCall` message containing the service name, method, and parameters as a `WindValue`.
    3.  The `RpcServer`'s `handle_client` loop receives the call. It looks up the method name in its `methods` `HashMap` and invokes the corresponding handler function (an `async` block).
    4.  The handler's result (`Ok(WindValue)` or `Err(String)`) is wrapped in a `MessagePayload::RpcResponse` and sent back to the client.

---

### 3. Tokio, Async, and Parallelism

This implementation uses Tokio to achieve high performance through a multi-threaded, event-driven architecture.

*   **Thread Creation**: Threads are not created manually. The `#[tokio::main]` macro initializes the **Tokio runtime**, which creates a pool of worker threads (one per CPU core). These threads are the foundation for all parallel execution in the application.

*   **Task Packaging (`tokio::spawn`)**: The unit of concurrency is a **task**, created with `tokio::spawn`. A task is a lightweight, asynchronous unit of work that the Tokio scheduler can run on any thread in its pool. This implementation spawns new tasks in several key places, enabling massive concurrency:
    1.  **`RegistryServer::run`**: When a new client connects (`listener.accept().await`), it spawns a `handle_client` task for that connection. This allows the registry to handle hundreds of clients simultaneously.
    2.  **`Publisher::start`**: Similarly, it spawns a `spawn_client_listener` task for each new subscriber.
    3.  **`RpcServer::start`**: It spawns a `handle_client` task for each new RPC client.
    4.  **`Publisher::publish`**: The `publish` method sends the new value to a broadcast channel. A single, dedicated background task (`start_update_sender`) listens on this channel and handles the fan-out to all subscribers. This decouples the act of publishing from the I/O of sending to clients.

*   **Async Intricacies and Parallelism**:
    *   **I/O-bound Concurrency**: The primary benefit of `async` is for I/O operations like reading/writing to a `TcpStream`. When a task `.await`s an I/O operation, it yields control back to the Tokio scheduler, allowing the thread to run other tasks instead of blocking. This is how a single thread can manage thousands of network connections.
    *   **True Parallelism**: Because the Tokio runtime uses a thread pool, different tasks (e.g., handling two different RPC clients) can run on different threads at the same time, achieving true parallelism and utilizing multiple CPU cores.
    *   **Shared State**: We use `Arc<RwLock<T>>` and `dashmap` to manage state shared between tasks. For example, the `RpcServer`'s `methods` `HashMap` is wrapped in `Arc<RwLock<...>>` so that multiple client-handling tasks can safely read from it in parallel. The `Registry` uses `DashMap` for a more granular, higher-performance concurrent hash map.

---

### 4. Possible Upgrades and Refinements

1.  **RPC Server Heartbeat**: The `RpcServer` registers itself once but, unlike the `Publisher`, does not have a heartbeat task to renew its registration. If the registry cleans up expired services, the `RpcServer` will become undiscoverable. We should add a heartbeat mechanism similar to the one in `Publisher`.

2.  **Decouple Publisher Client I/O**: In `Publisher::spawn_client_listener`, the logic to handle incoming messages (like `Subscribe`) is tightly coupled with the client connection. A more robust design would be to split the client's TCP stream into a reader and a writer half. One task would handle reading requests, and a separate task would handle writing outgoing data, likely using an `mpsc` channel for communication between the main publisher logic and the writer task. This isolates I/O concerns per client.

3.  **Connection Management in Client**: The `Connection` struct in the client reconnects on failure, which is good. However, the `RpcClient` and `Subscriber` create new connections for each action (`call`, `subscribe`). It would be more efficient to cache and reuse connections to services, only creating a new one if the existing connection is broken. We could manage a `HashMap` of active connections within the `WindClient`.

4.  **Centralized Server Logic**: The `WindServer` struct is a good start for a unified server, but the `Publisher` and `RpcServer` still run on different ports. A more advanced implementation could have a single server that binds to one port and demultiplexes incoming messages to either the Pub/Sub or RPC logic based on the message payload, simplifying deployment.

5.  **Graceful Shutdown**: The server loops (`loop { listener.accept().await }`) run indefinitely. Implementing a graceful shutdown mechanism using `tokio::signal` to listen for `Ctrl+C` would allow the servers to close connections cleanly and unregister from the registry before exiting.