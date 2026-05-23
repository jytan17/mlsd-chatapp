# default recipe: list all available commands when you just type `just`
default:
    @just --list

# --- infra (docker compose) ---

# start postgres + redis in background
up:
    docker compose up -d

# stop containers but keep data volumes
down:
    docker compose down

# stop AND delete data volumes (full reset)
nuke:
    docker compose down -v

# follow logs from both containers
logs:
    docker compose logs -f

# --- server ---

# compile + run the server crate (loads .env via dotenvy)
run:
    cargo run -p server

# compile all workspace crates without running
build:
    cargo build

# fast type-check only, no codegen (use while editing)
check:
    cargo check

# --- endpoint smoke tests ---

# hit /health, prints "ok" if process alive
health:
    curl -s http://localhost:3000/health && echo

# hit /ready, prints HTTP status code (200 healthy, 503 deps down)
ready:
    curl -s -o /dev/null -w "%{http_code}\n" http://localhost:3000/ready

# --- db shells ---

# drop into psql shell connected to the chat db
psql:
    docker exec -it mlsd-postgres psql -U chat -d chat

# drop into redis-cli connected to local redis
redis:
    docker exec -it mlsd-redis redis-cli
