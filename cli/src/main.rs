use clap::{Parser, Subcommand};
use std::{
    env,
    future::Future,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};

pub mod development {
    tonic::include_proto!("development");
}
use development::development_client::DevelopmentClient;
use development::{EchoReply, EchoRequest, Empty, Success};

type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the entire Mycelia project
    /// Shortcut for: `cargo build --workspace && cargo xtask build`
    Build,
    /// Start the Mycelia development server
    Start {
        /// The domain to listen on.
        /// Default: localhost
        /// TODO: add support to override (both here and in the development_server)
        #[clap(short, long, default_value = "127.0.0.1")]
        domain: String,
        /// The port http server should bind to.
        /// Default: 3001
        /// TODO: add support to override (both here and in the development_server)
        #[clap(long, default_value = "3001")]
        http_port: u16,

        /// The port rpc server should bind to
        /// Default: 50051
        /// TODO: add support to override (both here and in the development_server)
        #[clap(long, default_value = "50051")]
        rpc_port: u16,

        /// Open the development server in your default browser after starting.
        /// Default: true
        /// Possible values: true, false
        #[clap(short, long, default_value = "true")]
        open_browser: bool,
        // TODO: add browser override list
    },
    /// Stop the Mycelia development server
    Stop {
        /// The domain to listen on.
        /// Default: localhost
        /// TODO: add support to override (both here and in the development_server)
        #[clap(short, long, default_value = "127.0.0.1")]
        domain: String,

        /// The port rpc server should bind to
        /// Default: 50051
        /// TODO: add support to override (both here and in the development_server)
        #[clap(long, default_value = "50051")]
        rpc_port: u16,
    },
    // TODO: rm
    Print,
    /// Deploy your Mycelia project
    Deploy,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

struct LSPClient {
    _process: Child,
}

fn build() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = std::process::Command::new(cargo)
        .current_dir(project_root())
        .args(&["xtask", "build"])
        .status()?;

    if !status.success() {
        Err(format!(
            "`cargo xtask build` failed.

Status code: {}",
            status.code().unwrap()
        ))?;
    }

    Ok(())
}

async fn server_listening(address: String) -> Result<(), DynError> {
    // TODO: return https://doc.rust-lang.org/stable/std/task/enum.Poll.html# ?
    // TODO: use rand string?
    let echo = "poll_dev_server".to_string();

    let client = DevelopmentClient::connect(address.clone());
    if let Err(e) = client.await {
        match e.to_string().as_str() {
            "transport error" => {
                println!("Server not yet started");
                return Ok(());
            }
            _ => Err(e.into()),
        }
    } else {
        // FIXME: prevent duplicate ::connect
        let mut client = DevelopmentClient::connect(address.clone()).await?;
        let message = EchoRequest {
            message: echo.clone(),
        };
        let request = tonic::Request::new(message);
        let response = client.echo(request).await?;

        match response.into_inner() {
            EchoReply { message } => {
                if message == echo {
                    println!("Development server already listening");
                    return Err("Development server already listening".into());
                } else {
                    println!("Error echoing message to RPC server");
                    return Ok(());
                }
            }
        }
    }
}

async fn poll_server_listening() -> Result<(), DynError> {
    let start = tokio::time::Instant::now();
    let timeout = std::time::Duration::from_secs(5);

    loop {
        if let Err(_) = server_listening("http://127.0.0.1:50051".to_string()).await {
            println!("Development server listening x");
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        if start.elapsed() > timeout {
            Err("Timeout waiting for server to start")?;
        }
    }
}

async fn trigger(domain: &String, http_port: &u16, rpc_port: &u16, open_browser: &bool) {
    let (_client, tx_send, _tx_recv, wait) =
        start_server(domain, http_port, rpc_port, open_browser);

    tx_send.send("initialize".as_bytes().to_vec()).ok();

    // Wait for the handlers to exit. Currently this will never happen
    wait.await
}

fn start_server(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
) -> (
    LSPClient,
    UnboundedSender<Vec<u8>>,
    UnboundedSender<Vec<u8>>,
    impl Future<Output = ()>,
) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut process = Command::new(cargo)
        .current_dir(project_root())
        .args(&[
            "run",
            "--package=development_server",
            "--",
            format!("--http-port={}", http_port).as_str(),
            format!("--rpc-port={}", rpc_port).as_str(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("Unable to spawn process");

    let proc_id = process.id().expect("Unable to fetch process id");
    println!("proc id: {}", proc_id);

    let stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();
    // let stderr = process.stderr.take().unwrap();

    let writer = BufWriter::new(stdin);
    let reader = BufReader::new(stdout);

    let (tx_send, rx_send) = unbounded_channel::<Vec<u8>>();
    let (tx_recv, rx_recv) = unbounded_channel::<Vec<u8>>();

    println!("setting up tx,rx");
    // This future waits for handlers to exit, we don't want to await it here.
    // Return it instead so the caller can await it.
    let wait = setup_listeners(reader, writer, rx_send, rx_recv);

    // Send initialize request
    println!("initialize request");
    tx_send.send("initialize".as_bytes().to_vec()).ok();
    let client = LSPClient { _process: process };

    (client, tx_send, tx_recv, wait)
}

async fn setup_listeners(
    mut reader: BufReader<ChildStdout>,
    mut writer: BufWriter<ChildStdin>,
    mut rx_send: UnboundedReceiver<Vec<u8>>,
    mut rx_recv: UnboundedReceiver<Vec<u8>>,
) {
    let handle_recv = tokio::spawn(async move {
        loop {
            // Wait until a message is available instead of constantly polling for a message.
            match rx_recv.recv().await {
                Some(data) => {
                    println!("rx_recv got: {:?}", String::from_utf8(data));

                    let mut buf = String::new();
                    let _ = reader.read_line(&mut buf);
                }
                None => {
                    println!("rx_recv: quitting loop");
                    break;
                }
            }
        }
    });

    let handle_send = tokio::spawn(async move {
        loop {
            // Wait until a message is available instead of constantly polling for a message.
            match rx_send.recv().await {
                Some(data) => {
                    println!("rx_send: got something");
                    let str = String::from_utf8(data).expect("invalid data for parse");
                    println!("rx_send: sending to LSP: {:?}", str);

                    _ = writer.write_all(str.as_bytes());
                }
                None => {
                    println!("rx_recv: quitting loop");
                    break;
                }
            }
        }
    });

    // Wait for a handler to exit. You already spawned the handlers, you don't need to spawn them again.
    // I'm using select instead of join so we can see any errors immediately.
    tokio::select! {
       send = handle_send => println!("{send:?}"),
       recv = handle_recv => println!("{recv:?}"),
    };
}

async fn start(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
) -> Result<(), DynError> {
    // TODO: might wanna move these prints to the development_server
    let http_addr = format!("http://{}:{}", domain, http_port);
    let rpc_addr = format!("http://{}:{}", domain, rpc_port);

    // TODO: handle Error from server_listening
    if let Ok(_) = server_listening(rpc_addr.clone()).await {
        trigger(domain, http_port, rpc_port, open_browser).await;

        println!("HTTP development server listening on {}", http_addr);
        println!("RPC server listening on {}", rpc_addr);
    }

    if *open_browser {
        poll_server_listening().await?;
        match open::that(&http_addr) {
            Ok(()) => println!("Opened '{}' in your default browser.", http_addr),
            Err(err) => eprintln!("An error occurred when opening '{}': {}", http_addr, err),
        }
    } else {
        println!("You can reach the development server on {}", http_addr);
    }

    Ok(())
}

async fn stop(domain: &str, rpc_port: &u16) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = try_stop(domain, rpc_port).await {
        eprintln!("{}", e);

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_stop(domain: &str, rpc_port: &u16) -> Result<(), DynError> {
    println!("Stopping development server");
    let address = format!("http://{}:{}", domain, rpc_port);
    let client = DevelopmentClient::connect(address.clone()).await;
    match client {
        Ok(_) => {
            let mut client = DevelopmentClient::connect(address.clone()).await?;
            let request = tonic::Request::new(Empty {});
            let response = client.stop_server(request).await?;

            match response.into_inner() {
                Success {} => {
                    println!("Successfully stopped development server");
                }
            }
        }
        Err(e) => {
            if e.to_string() == "transport error" {
                println!("Server already stopped");
                return Ok(());
            } else {
                return Err(e.into());
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);

        // std::process::exit(-1);
    }

    Ok(())
}

async fn try_main() -> Result<(), DynError> {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Build => {
            build()?;
        }
        Commands::Start {
            domain,
            http_port,
            rpc_port,
            open_browser,
        } => {
            let _ = start(domain, http_port, rpc_port, open_browser).await?;
        }
        Commands::Stop { domain, rpc_port } => {
            let _ = stop(domain, rpc_port).await;
        }
        Commands::Print {} => {
            println!("TODO: print");
        }
        Commands::Deploy => {
            println!("TODO: deploy");
        }
    }

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
