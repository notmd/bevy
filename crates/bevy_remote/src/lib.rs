//! An implementation of the Bevy Remote Protocol over HTTP and JSON, to allow
//! for remote control of a Bevy app.
//!
//! Adding the [`RemotePlugin`] to your [`App`] causes Bevy to accept
//! connections over HTTP (by default, on port 15702) while your app is running.
//! These *remote clients* can inspect and alter the state of the
//! entity-component system. Clients are expected to `POST` JSON requests to the
//! root URL; see the `client` example for a trivial example of use.
//!
//! The Bevy Remote Protocol is based on the JSON-RPC 2.0 protocol.
//!
//! ## Request objects
//!
//! A typical client request might look like this:
//!
//! ```json
//! {
//!     "method": "bevy/get",
//!     "id": 0,
//!     "params": {
//!         "entity": 4294967298,
//!         "components": [
//!             "bevy_transform::components::transform::Transform"
//!         ]
//!     }
//! }
//! ```
//!
//! The `id` and `method` fields are required. The `params` field may be omitted
//! for certain methods:
//!
//! * `id` is arbitrary JSON data. The server completely ignores its contents,
//!   and the client may use it for any purpose. It will be copied via
//!   serialization and deserialization (so object property order, etc. can't be
//!   relied upon to be identical) and sent back to the client as part of the
//!   response.
//!
//! * `method` is a string that specifies one of the possible [`BrpRequest`]
//!   variants: `bevy/query`, `bevy/get`, `bevy/insert`, etc. It's case-sensitive.
//!
//! * `params` is parameter data specific to the request.
//!
//! For more information, see the documentation for [`BrpRequest`].
//! [`BrpRequest`] is serialized to JSON via `serde`, so [the `serde`
//! documentation] may be useful to clarify the correspondence between the Rust
//! structure and the JSON format.
//!
//! ## Response objects
//!
//! A response from the server to the client might look like this:
//!
//! ```json
//! {
//!     "jsonrpc": "2.0",
//!     "id": 0,
//!     "result": {
//!         "bevy_transform::components::transform::Transform": {
//!             "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
//!             "scale": { "x": 1.0, "y": 1.0, "z": 1.0 },
//!             "translation": { "x": 0.0, "y": 0.5, "z": 0.0 }
//!         }
//!     }
//! }
//! ```
//!
//! The `id` field will always be present. The `result` field will be present if the
//! request was successful. Otherwise, an `error` field will replace it.
//!
//! * `id` is the arbitrary JSON data that was sent as part of the request. It
//!   will be identical to the `id` data sent during the request, modulo
//!   serialization and deserialization. If there's an error reading the `id` field,
//!   it will be `null`.
//!
//! * `result` will be present if the request succeeded and will contain the response
//!   specific to the request.
//!
//! * `error` will be present if the request failed and will contain an error object
//!   with more information about the cause of failure.
//!
//! ## Error objects
//!
//! An error object might look like this:
//!
//! ```json
//! {
//!     "code": -32602,
//!     "message": "Missing \"entity\" field"
//! }
//! ```
//!
//! The `code` and `message` fields will always be present. There may also be a `data` field.
//!
//! * `code` is an integer representing the kind of an error that happened. Error codes documented
//!   in the [`error_codes`] module.
//!
//! * `message` is a short, one-sentence human-readable description of the error.
//!
//! * `data` is an optional field of arbitrary type containing additional information about the error.
//!
//! ## Built-in methods
//!
//! The Bevy Remote Protocol includes a number of built-in methods for accessing and modifying data
//! in the ECS. Each of these methods uses the `bevy/` prefix, which is a namespace reserved for
//! BRP built-in methods.
//!
//! ### bevy/get
//!
//! Retrieve the values of one or more components from an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity whose components will be fetched.
//! - `components`: An array of [fully-qualified type names] of components to fetch.
//!
//! `result`: A map associating each type name to its value on the requested entity.
//!
//! ### bevy/query
//!
//! Perform a query over components in the ECS, returning all matching entities and their associated
//! component values.
//!
//! All of the arrays that comprise this request are optional, and when they are not provided, they
//! will be treated as if they were empty.
//!
//! `params`:
//! - `data`:
//!   - `components` (optional): An array of [fully-qualified type names] of components to fetch.
//!   - `option` (optional): An array of fully-qualified type names of components to fetch optionally.
//!   - `has` (optional): An array of fully-qualified type names of components whose presence will be
//!      reported as boolean values.
//! - `filter` (optional):
//!   - `with` (optional): An array of fully-qualified type names of components that must be present
//!     on entities in order for them to be included in results.
//!   - `without` (optional): An array of fully-qualified type names of components that must *not* be
//!     present on entities in order for them to be included in results.
//!
//! `result`: An array, each of which is an object containing:
//! - `entity`: The ID of a query-matching entity.
//! - `components`: A map associating each type name from `components`/`option` to its value on the matching
//!   entity if the component is present.
//! - `has`: A map associating each type name from `has` to a boolean value indicating whether or not the
//!   entity has that component. If `has` was empty or omitted, this key will be omitted in the response.
//!
//! ### bevy/spawn
//!
//! Create a new entity with the provided components and return the resulting entity ID.
//!
//! `params`:
//! - `components`: A map associating each component's [fully-qualified type name] with its value.
//!
//! `result`:
//! - `entity`: The ID of the newly spawned entity.
//!
//! ### bevy/destroy
//!
//! Despawn the entity with the given ID.
//!
//! `params`:
//! - `entity`: The ID of the entity to be despawned.
//!
//! `result`: null.
//!
//! ### bevy/remove
//!
//! Delete one or more components from an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity whose components should be removed.
//! - `components`: An array of [fully-qualified type names] of components to be removed.
//!
//! `result`: null.
//!
//! ### bevy/insert
//!
//! Insert one or more components into an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity to insert components into.
//! - `components`: A map associating each component's fully-qualified type name with its value.
//!
//! `result`: null.
//!
//! ### bevy/reparent
//!
//! Assign a new parent to one or more entities.
//!
//! `params`:
//! - `entities`: An array of entity IDs of entities that will be made children of the `parent`.
//! - `parent` (optional): The entity ID of the parent to which the child entities will be assigned.
//!   If excluded, the given entities will be removed from their parents.
//!
//! `result`: null.
//!
//! ### bevy/list
//!
//! List all registered components or all components present on an entity.
//!
//! When `params` is not provided, this lists all registered components. If `params` is provided,
//! this lists only those components present on the provided entity.
//!
//! `params` (optional):
//! - `entity`: The ID of the entity whose components will be listed.
//!
//! `result`: An array of fully-qualified type names of components.
//!
//! ## Custom methods
//!
//! In addition to the provided methods, the Bevy Remote Protocol can be extended to include custom
//! methods. This is primarily done during the initialization of [`RemotePlugin`], although the
//! methods may also be extended at runtime using the [`RemoteMethods`] resource.
//!
//! ### Example
//! ```ignore
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(
//!             // `default` adds all of the built-in methods, while `with_method` extends them
//!             RemotePlugin::default()
//!                 .with_method("super_user/cool_method", path::to::my::cool::handler)
//!                 // ... more methods can be added by chaining `with_method`
//!         )
//!         .add_systems(
//!             // ... standard application setup
//!         )
//!         .run();
//! }
//! ```
//!
//! The handler is expected to be a system-convertible function which takes optional JSON parameters
//! as input and returns a [`BrpResult`]. This means that it should have a type signature which looks
//! something like this:
//! ```
//! # use serde_json::Value;
//! # use bevy_ecs::prelude::{In, World};
//! # use bevy_remote::BrpResult;
//! fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
//!     todo!()
//! }
//! ```
//!
//! Arbitrary system parameters can be used in conjunction with the optional `Value` input. The
//! handler system will always run with exclusive `World` access.
//!
//! [the `serde` documentation]: https://serde.rs/
//! [fully-qualified type names]: bevy_reflect::TypePath::type_path
//! [fully-qualified type name]: bevy_reflect::TypePath::type_path

#![cfg(not(target_family = "wasm"))]

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::RwLock,
};

use anyhow::Result as AnyhowResult;
use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, In, IntoSystem, Res, Resource, System, SystemId},
    world::{Mut, World},
};
use bevy_reflect::Reflect;
use bevy_tasks::IoTaskPool;
use bevy_utils::{prelude::default, HashMap};
use futures_util::SinkExt;
use http_body_util::{BodyExt as _, Full};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service, Request, Response,
};
use hyper_tungstenite::HyperWebsocket;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use smol::{
    channel::{self, Receiver, Sender},
    Async,
};
use smol_hyper::rt::{FuturesIo, SmolTimer};
use std::net::{TcpListener, TcpStream};

pub mod builtin_methods;
pub mod error_codes;

/// The default port that Bevy will listen on.
///
/// This value was chosen randomly.
pub const DEFAULT_PORT: u16 = 15702;

/// The default host address that Bevy will use for its server.
pub const DEFAULT_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

const CHANNEL_SIZE: usize = 16;

/// Add this plugin to your [`App`] to allow remote connections to inspect and modify entities.
/// This the main plugin for `bevy_remote`. See the [crate-level documentation] for details on
/// the protocol and its default methods.
///
/// The defaults are:
/// - [`DEFAULT_ADDR`] : 127.0.0.1.
/// - [`DEFAULT_PORT`] : 15702.
///
/// [crate-level documentation]: crate
pub struct RemotePlugin {
    /// The address that Bevy will use.
    address: IpAddr,

    /// The port that Bevy will listen on.
    port: u16,

    /// The verbs that the server will recognize and respond to.
    methods: RwLock<
        Vec<(
            String,
            Box<dyn System<In = In<Option<Value>>, Out = BrpResult>>,
        )>,
    >,

    streaming_methods: RwLock<
        Vec<(
            String,
            Box<dyn System<In = In<Option<Value>>, Out = Option<BrpResult>>>,
        )>,
    >,
}

impl RemotePlugin {
    /// Create a [`RemotePlugin`] with the default address and port but without
    /// any associated methods.
    fn empty() -> Self {
        Self {
            address: DEFAULT_ADDR,
            port: DEFAULT_PORT,
            methods: RwLock::new(vec![]),
            streaming_methods: RwLock::new(vec![]),
        }
    }

    /// Set the IP address that the server will use.
    #[must_use]
    pub fn with_address(mut self, address: impl Into<IpAddr>) -> Self {
        self.address = address.into();
        self
    }

    /// Set the remote port that the server will listen on.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Add a normal remote method to the plugin using the given `name` and `handler`.
    #[must_use]
    pub fn with_method<M>(
        mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult, M>,
    ) -> Self {
        self.methods
            .get_mut()
            .unwrap()
            .push((name.into(), Box::new(IntoSystem::into_system(handler))));
        self
    }

    /// Add a streaming remote method to the plugin using the given `name` and `handler`.
    /// The handler will be called every frame when there is a client connected to the stream.
    /// The handler should return a `None` to indicate that there is nothing to stream.
    /// And return `Some(BrpErr)` to stop the stream.
    #[must_use]
    pub fn with_stream_method<M>(
        mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, Option<BrpResult>, M>,
    ) -> Self {
        self.streaming_methods
            .get_mut()
            .unwrap()
            .push((name.into(), Box::new(IntoSystem::into_system(handler))));
        self
    }
}

impl Default for RemotePlugin {
    fn default() -> Self {
        Self::empty()
            .with_method(
                builtin_methods::BRP_GET_METHOD,
                builtin_methods::process_remote_get_request,
            )
            .with_method(
                builtin_methods::BRP_QUERY_METHOD,
                builtin_methods::process_remote_query_request,
            )
            .with_method(
                builtin_methods::BRP_SPAWN_METHOD,
                builtin_methods::process_remote_spawn_request,
            )
            .with_method(
                builtin_methods::BRP_INSERT_METHOD,
                builtin_methods::process_remote_insert_request,
            )
            .with_method(
                builtin_methods::BRP_REMOVE_METHOD,
                builtin_methods::process_remote_remove_request,
            )
            .with_method(
                builtin_methods::BRP_DESTROY_METHOD,
                builtin_methods::process_remote_destroy_request,
            )
            .with_method(
                builtin_methods::BRP_REPARENT_METHOD,
                builtin_methods::process_remote_reparent_request,
            )
            .with_method(
                builtin_methods::BRP_LIST_METHOD,
                builtin_methods::process_remote_list_request,
            )
    }
}

impl Plugin for RemotePlugin {
    fn build(&self, app: &mut App) {
        let mut remote_methods = RemoteMethods::new();
        let plugin_methods = &mut *self.methods.write().unwrap();
        for (name, system) in plugin_methods.drain(..) {
            remote_methods.insert(
                name,
                RemoteMethod::Normal(app.main_mut().world_mut().register_boxed_system(system)),
            );
        }

        let plugin_methods = &mut *self.streaming_methods.write().unwrap();

        for (name, system) in plugin_methods.drain(..) {
            remote_methods.insert(
                name.clone(),
                RemoteMethod::Stream(app.main_mut().world_mut().register_boxed_system(system)),
            );
        }

        app.insert_resource(HostAddress(self.address))
            .insert_resource(HostPort(self.port))
            .insert_resource(remote_methods)
            .add_systems(Startup, start_server)
            .add_systems(Update, process_remote_requests);
    }
}

/// A resource containing the IP address that Bevy will host on.
///
/// Currently, changing this while the application is running has no effect; this merely
/// reflects the IP address that is set during the setup of the [`RemotePlugin`].
#[derive(Debug, Resource)]
pub struct HostAddress(pub IpAddr);

/// A resource containing the port number that Bevy will listen on.
///
/// Currently, changing this while the application is running has no effect; this merely
/// reflects the host that is set during the setup of the [`RemotePlugin`].
#[derive(Debug, Resource, Reflect)]
pub struct HostPort(pub u16);

/// The type of a function that implements a remote method (`bevy/get`, `bevy/query`, etc.)
///
/// The first parameter is the JSON value of the `params`. Typically, an
/// implementation will deserialize these as the first thing they do.
///
/// The returned JSON value will be returned as the response. Bevy will
/// automatically populate the `id` field before sending.
#[derive(Debug, Clone)]
pub enum RemoteMethod {
    /// A normal method that is called once per request.
    Normal(SystemId<In<Option<Value>>, BrpResult>),
    /// A streaming method that is called every frame while a client is connected.
    Stream(SystemId<In<Option<Value>>, Option<BrpResult>>),
}

/// Holds all implementations of methods known to the server.
///
/// Custom methods can be added to this list using [`RemoteMethods::insert`].
#[derive(Debug, Resource, Default)]
pub struct RemoteMethods(HashMap<String, RemoteMethod>);

impl RemoteMethods {
    /// Creates a new [`RemoteMethods`] resource with no methods registered in it.
    pub fn new() -> Self {
        default()
    }

    /// Adds a new method, replacing any existing method with that name.
    ///
    /// If there was an existing method with that name, returns its handler.
    pub fn insert(
        &mut self,
        method_name: impl Into<String>,
        handler: RemoteMethod,
    ) -> Option<RemoteMethod> {
        self.0.insert(method_name.into(), handler)
    }
}

/// A single request from a Bevy Remote Protocol client to the server,
/// serialized in JSON.
///
/// The JSON payload is expected to look like this:
///
/// ```json
/// {
///     "jsonrpc": "2.0",
///     "method": "bevy/get",
///     "id": 0,
///     "params": {
///         "entity": 4294967298,
///         "components": [
///             "bevy_transform::components::transform::Transform"
///         ]
///     }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpRequest {
    /// This field is mandatory and must be set to `"2.0"` for the request to be accepted.
    pub jsonrpc: String,

    /// The action to be performed.
    pub method: String,

    /// Arbitrary data that will be returned verbatim to the client as part of
    /// the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// The parameters, specific to each method.
    ///
    /// These are passed as the first argument to the method handler.
    /// Sometimes params can be omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A response according to BRP.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpResponse {
    /// This field is mandatory and must be set to `"2.0"`.
    pub jsonrpc: &'static str,

    /// The id of the original request.
    pub id: Option<Value>,

    /// The actual response payload.
    #[serde(flatten)]
    pub payload: BrpPayload,
}

impl BrpResponse {
    /// Generates a [`BrpResponse`] from an id and a `Result`.
    #[must_use]
    pub fn new(id: Option<Value>, result: BrpResult) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            payload: BrpPayload::from(result),
        }
    }
}

/// A result/error payload present in every response.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BrpPayload {
    /// `Ok` variant
    Result(Value),
    /// `Err` variant
    Error(BrpError),
}

impl From<BrpResult> for BrpPayload {
    fn from(value: BrpResult) -> Self {
        match value {
            Ok(v) => Self::Result(v),
            Err(err) => Self::Error(err),
        }
    }
}

/// An error a request might return.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpError {
    /// Defines the general type of the error.
    pub code: i16,
    /// Short, human-readable description of the error.
    pub message: String,
    /// Optional additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl BrpError {
    /// Entity wasn't found.
    #[must_use]
    pub fn entity_not_found(entity: Entity) -> Self {
        Self {
            code: error_codes::ENTITY_NOT_FOUND,
            message: format!("Entity {entity} not found"),
            data: None,
        }
    }

    /// Component wasn't found in an entity.
    #[must_use]
    pub fn component_not_present(component: &str, entity: Entity) -> Self {
        Self {
            code: error_codes::COMPONENT_NOT_PRESENT,
            message: format!("Component `{component}` not present in Entity {entity}"),
            data: None,
        }
    }

    /// An arbitrary component error. Possibly related to reflection.
    #[must_use]
    pub fn component_error<E: ToString>(error: E) -> Self {
        Self {
            code: error_codes::COMPONENT_ERROR,
            message: error.to_string(),
            data: None,
        }
    }

    /// An arbitrary internal error.
    #[must_use]
    pub fn internal<E: ToString>(error: E) -> Self {
        Self {
            code: error_codes::INTERNAL_ERROR,
            message: error.to_string(),
            data: None,
        }
    }

    /// Attempt to reparent an entity to itself.
    #[must_use]
    pub fn self_reparent(entity: Entity) -> Self {
        Self {
            code: error_codes::SELF_REPARENT,
            message: format!("Cannot reparent Entity {entity} to itself"),
            data: None,
        }
    }
}

/// The result of a request.
pub type BrpResult = Result<Value, BrpError>;

/// The requests may occur on their own or in batches.
/// Actual parsing is deferred for the sake of proper
/// error reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BrpBatch {
    /// Multiple requests with deferred parsing.
    Batch(Vec<Value>),
    /// A single request with deferred parsing.
    Single(Value),
}

/// A message from the Bevy Remote Protocol server thread to the main world.
///
/// This is placed in the [`BrpMailbox`].
#[derive(Debug, Clone)]
pub struct BrpMessage {
    /// The request method.
    pub method: String,

    /// The request params.
    pub params: Option<Value>,

    /// The channel on which the response is to be sent.
    ///
    /// The value sent here is serialized and sent back to the client.
    pub sender: Sender<BrpResult>,
}

/// A resource that receives messages sent by Bevy Remote Protocol clients.
///
/// Every frame, the `process_remote_requests` system drains this mailbox and
/// processes the messages within.
#[derive(Debug, Resource, Deref, DerefMut)]
pub struct BrpMailbox(Receiver<BrpMessage>);

#[derive(Debug, Component, Clone)]
struct ActiveStream(BrpMessage, RemoteMethod);

/// A system that starts up the Bevy Remote Protocol server.
fn start_server(mut commands: Commands, address: Res<HostAddress>, remote_port: Res<HostPort>) {
    // Create the channel and the mailbox.
    let (request_sender, request_receiver) = channel::bounded(CHANNEL_SIZE);
    commands.insert_resource(BrpMailbox(request_receiver));

    IoTaskPool::get()
        .spawn(server_main(address.0, remote_port.0, request_sender))
        .detach();
}

/// A system that receives requests placed in the [`BrpMailbox`] and processes
/// them, using the [`RemoteMethods`] resource to map each request to its handler.
///
/// This needs exclusive access to the [`World`] because clients can manipulate
/// anything in the ECS.
fn process_remote_requests(world: &mut World) {
    if !world.contains_resource::<BrpMailbox>() {
        return;
    }

    while let Ok(message) = world.resource_mut::<BrpMailbox>().try_recv() {
        world.resource_scope(|world, methods: Mut<RemoteMethods>| {
            // Fetch the handler for the method. If there's no such handler
            // registered, return an error.
            let Some(handler) = methods.0.get(&message.method) else {
                let _ = message.sender.send_blocking(Err(BrpError {
                    code: error_codes::METHOD_NOT_FOUND,
                    message: format!("Method `{}` not found", message.method),
                    data: None,
                }));
                return;
            };

            let result = match handler {
                RemoteMethod::Normal(system_id) => {
                    world.run_system_with_input(*system_id, message.params)
                }
                RemoteMethod::Stream(system_id) => {
                    world.spawn(ActiveStream(
                        message.clone(),
                        RemoteMethod::Stream(*system_id),
                    ));

                    return;
                }
            };

            // Execute the handler, and send the result back to the client.
            let result = match result {
                Ok(result) => result,
                Err(error) => {
                    let _ = message.sender.send_blocking(Err(BrpError {
                        code: error_codes::INTERNAL_ERROR,
                        message: format!("Failed to run method handler: {error}"),
                        data: None,
                    }));
                    return;
                }
            };

            let _ = message.sender.send_blocking(result);
        });
    }

    let streams: Vec<_> = world
        .query::<(Entity, &ActiveStream)>()
        .iter(world)
        .map(|item| (item.0, item.1.clone()))
        .collect();

    let to_remove: Vec<_> = streams
        .into_iter()
        .filter_map(|(entity, stream)| match stream.1 {
            RemoteMethod::Stream(system_id) => {
                let message = stream.0;
                let result = world.run_system_with_input(system_id, message.params);

                let should_remove = match result {
                    Ok(handler_result) => {
                        if let Some(handler_result) = handler_result {
                            let handler_err = handler_result.is_err();
                            let channel_result = message.sender.send_blocking(handler_result);

                            // Remove the entity when the handler return error or channel closed
                            handler_err || channel_result.is_err()
                        } else {
                            false
                        }
                    }
                    Err(error) => {
                        let _ = message.sender.send_blocking(Err(BrpError {
                            code: error_codes::INTERNAL_ERROR,
                            message: format!("Failed to run method handler: {error}"),
                            data: None,
                        }));

                        true
                    }
                };

                should_remove.then_some(entity)
            }
            _ => unreachable!(),
        })
        .collect();

    for entity in to_remove {
        world.despawn(entity);
    }
}

/// The Bevy Remote Protocol server main loop.
async fn server_main(
    address: IpAddr,
    port: u16,
    request_sender: Sender<BrpMessage>,
) -> AnyhowResult<()> {
    listen(
        Async::<TcpListener>::bind((address, port))?,
        &request_sender,
    )
    .await
}

async fn listen(
    listener: Async<TcpListener>,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<()> {
    loop {
        let (client, _) = listener.accept().await?;
        let request_sender = request_sender.clone();
        IoTaskPool::get()
            .spawn(async move {
                let _ = handle_client(client, request_sender).await;
            })
            .detach();
    }
}

async fn handle_client(
    client: Async<TcpStream>,
    request_sender: Sender<BrpMessage>,
) -> AnyhowResult<()> {
    http1::Builder::new()
        .keep_alive(true)
        .timer(SmolTimer::new())
        .serve_connection(
            FuturesIo::new(client),
            service::service_fn(|request| process_request(request, &request_sender)),
        )
        .with_upgrades()
        .await?;

    Ok(())
}

async fn process_request(
    mut request: Request<Incoming>,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<Response<Full<Bytes>>> {
    if hyper_tungstenite::is_upgrade_request(&request) {
        let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;
        let body = match validate_websocket_request(&request) {
            Ok(body) => body,
            Err(err) => {
                let response = serde_json::to_string(&BrpError {
                    code: error_codes::INVALID_REQUEST,
                    message: format!("{err}"),
                    data: None,
                })?;

                return Ok(Response::new(Full::new(response.into_bytes().into())));
            }
        };

        let request_sender = request_sender.clone();

        IoTaskPool::get()
            .spawn(async move { process_brp_websocket(websocket, request_sender, body).await })
            .detach();

        return Ok(response);
    }
    let batch_bytes = request.into_body().collect().await?.to_bytes();
    let serialized = process_brp_batch(batch_bytes, request_sender).await?;

    Ok(Response::new(Full::new(serialized)))
}

/// A helper function for the Bevy Remote Protocol server that handles a batch
/// of requests coming from a client.
async fn process_brp_batch(
    bytes: Bytes,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<Bytes> {
    let batch: Result<BrpBatch, _> = serde_json::from_slice(&bytes);
    let serialized = match batch {
        Ok(BrpBatch::Single(request)) => {
            serde_json::to_string(&process_single_request(request, request_sender).await?)?
        }
        Ok(BrpBatch::Batch(requests)) => {
            let mut responses = Vec::new();

            for request in requests {
                responses.push(process_single_request(request, request_sender).await?);
            }

            serde_json::to_string(&responses)?
        }
        Err(err) => serde_json::to_string(&BrpError {
            code: error_codes::INVALID_REQUEST,
            message: err.to_string(),
            data: None,
        })?,
    };

    Ok(Bytes::from(serialized.as_bytes().to_owned()))
}

/// A helper function for the Bevy Remote Protocol server that processes a single
/// request coming from a client.
async fn process_single_request(
    request: Value,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<BrpResponse> {
    // Reach in and get the request ID early so that we can report it even when parsing fails.
    let id = request.as_object().and_then(|map| map.get("id")).cloned();

    let request: BrpRequest = match serde_json::from_value(request) {
        Ok(v) => v,
        Err(err) => {
            return Ok(BrpResponse::new(
                id,
                Err(BrpError {
                    code: error_codes::INVALID_REQUEST,
                    message: err.to_string(),
                    data: None,
                }),
            ));
        }
    };

    if request.jsonrpc != "2.0" {
        return Ok(BrpResponse::new(
            id,
            Err(BrpError {
                code: error_codes::INVALID_REQUEST,
                message: String::from("JSON-RPC request requires `\"jsonrpc\": \"2.0\"`"),
                data: None,
            }),
        ));
    }

    let (result_sender, result_receiver) = channel::bounded(1);

    let _ = request_sender
        .send(BrpMessage {
            method: request.method,
            params: request.params,
            sender: result_sender,
        })
        .await;

    let result = result_receiver.recv().await?;
    Ok(BrpResponse::new(request.id, result))
}

async fn process_brp_websocket(
    websocket: HyperWebsocket,
    request_sender: Sender<BrpMessage>,
    request: BrpRequest,
) -> AnyhowResult<()> {
    let mut websocket = websocket.await?;

    let (result_sender, result_receiver) = channel::bounded(1);

    let id = request.id;

    let _ = request_sender
        .send(BrpMessage {
            method: request.method,
            params: request.params,
            sender: result_sender,
        })
        .await;

    while let Ok(result) = result_receiver.recv().await {
        let response = serde_json::to_string(&BrpResponse::new(id.clone(), result))?;
        websocket.send(tungstenite::Message::text(response)).await?;
    }

    Ok(())
}

fn validate_websocket_request(request: &Request<Incoming>) -> AnyhowResult<BrpRequest> {
    let body = request
        .uri()
        .query()
        .map(|query| {
            // Simple query string parsing
            let mut map = HashMap::new();
            for pair in query.split('&') {
                let mut it = pair.split('=').take(2);
                let (Some(k), Some(v)) = (it.next(), it.next()) else {
                    continue;
                };
                map.insert(k, v);
            }
            map
        })
        .and_then(|query| query.get("body").cloned())
        .ok_or_else(|| anyhow::anyhow!("Missing body"))?;

    let body = urlencoding::decode(body)?.into_owned();
    let batch = serde_json::from_str(&body).map_err(|err| anyhow::anyhow!(err))?;

    let body = match batch {
        BrpBatch::Batch(_vec) => {
            anyhow::bail!("Batch requests are not supported for streaming")
        }
        BrpBatch::Single(value) => value,
    };

    match serde_json::from_value::<BrpRequest>(body) {
        Ok(req) => {
            if req.jsonrpc != "2.0" {
                anyhow::bail!("JSON-RPC request requires `\"jsonrpc\": \"2.0\"`")
            }

            Ok(req)
        }
        Err(err) => anyhow::bail!(err),
    }
}
