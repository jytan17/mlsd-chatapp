run:
    cargo run -p server

db-up:
    docker compose up -d

db-down:
    docker compose down
