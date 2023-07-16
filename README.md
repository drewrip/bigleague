# bigleague

Web app that takes Sleeper leagues and turns them into a big league.

The end goal is to be able to run a league of 80+ players using Sleeper as the platform.

## Setup

Setting up bigleague should be quite simple. There are two required pieces, a Postgres instance and the bigleague server.

### Postgres Quickstart

A simple way to get started is with Postgres in a container. We'll be using `podman`, and installation instructions can be found [here](https://podman.io/docs/installation). It can be found in most modern package managers for easy install.

Start by pulling the official Postgres image from Docker:
```
podman pull docker.io/library/postgres:14
```

Launch a pod for Postgres to be placed in:
```
podman pod create --name postgres -p 5432:5432
```

Add a Postgres instance to the pod:
```
podman run -dt --pod postgres -e POSTGRES_DB=bigleague -e POSTGRES_USER=admin -e POSTGRES_PASSWORD=password postgres:14
```

This will expose Postgres under the default port 5432 for testing and development.

### Starting bigleague

Make sure that the `Bigleague.toml` configuration file points to your Postgres database:
```
...
user = "admin"
password = "password"
dbname = "bigleague"
...
```

Also, be sure to add a list of participating leagues (in the form of Sleeper league ids) under the `.bigleague.leagues` field.

Then we can start bigleague with `cargo`:
```
RUST_LOG=info cargo run
```

The `RUST_LOG` env variable sets the log level.
