FROM wdshihaoren/python_rt:build-1.75-240527 as build
WORKDIR /ai_agent/
#COPY build.toml .cargo/config.toml
#COPY vendor vendor
#COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo build --bin python_rt --release

FROM python:3.11-alpine
EXPOSE 50001
WORKDIR /ai_agent/
COPY --from=build /ai_agent/target/release/python_rt .

CMD ["./python_rt", "-t 4 --io --time"]