use bevy::prelude::*;
use bevy_tokio_tasks::*;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use tokio::task;

#[derive(Debug, Component)]
struct HyperListen(pub u16);

#[derive(Debug, Component)]
struct HyperTask(pub task::JoinHandle<Result<(), hyper::Error>>);

// TODO: how do we *stop* listeners?

async fn http_request_handler(
    req: Request<Body>,
    mut ctx: TaskContext,
) -> Result<Response<Body>, hyper::Error> {
    info!("handling request");

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            info!("got GET to '/'");

            ctx.run_on_main_thread(|_ctx| {
                info!("GET on the main thread!");
            })
            .await;

            Ok(Response::new("hello GET".into()))
        }
        (&Method::POST, "/") => {
            info!("got POST to '/'");

            ctx.run_on_main_thread(|_ctx| {
                info!("POST on the main thread!");
            })
            .await;

            Ok(Response::new("hello POST".into()))
        }
        _ => {
            info!("not found");

            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

fn start_http_listener(
    mut commands: Commands,
    mut requests: Query<(Entity, &mut HyperListen), Added<HyperListen>>,
    runtime: Res<TokioTasksRuntime>,
) {
    for (entity, request) in requests.iter_mut() {
        let port = request.0;

        let task = runtime.spawn_background_task(move |ctx| async move {
            let addr = ([127, 0, 0, 1], port).into();

            // TODO: how do I get ctx passed into this?
            // no amount of cloning it seems to pass the test
            // solution: https://users.rust-lang.org/t/hyper-tokio-pass-variable-to-service-handler/40550/2
            let service = make_service_fn(move |_| {
                let ctx = ctx.clone();
                async move {
                    Ok::<_, hyper::Error>(service_fn(move |req| {
                        let ctx = ctx.clone();
                        http_request_handler(req, ctx)
                    }))
                }
            });

            let server = Server::bind(&addr).serve(service);

            info!("Listening on http://{}", addr);

            server.await?;

            Ok(())
        });

        commands
            .entity(entity)
            .insert(HyperTask(task))
            .remove::<HyperListen>();
    }
}

struct HyperPlugin;

impl Plugin for HyperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((start_http_listener,));
    }
}

fn startup(mut commands: Commands) {
    commands.spawn(HyperListen(3000));
}

fn main() {
    App::new()
        .insert_resource(bevy::app::ScheduleRunnerSettings::run_loop(
            bevy::utils::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .add_plugins(MinimalPlugins)
        .add_plugin(bevy::log::LogPlugin {
            level: bevy::log::Level::INFO,
            ..Default::default()
        })
        .add_plugin(bevy_tokio_tasks::TokioTasksPlugin::default())
        .add_plugin(HyperPlugin)
        .add_startup_system(startup)
        .run();
}
