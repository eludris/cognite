# Cognite

:rocket: A simple blazingly fast language agnostic microservice driven Discord bot
framework made in rust that leverages Apache Kafka and KeyDB to take care of the
gateway and caching leaving the handling for you to do in your preferred
programming language with toolings to help you bring in your preferred Database of
choice and presets is some langauges to help you get started.

Everything in Cognite is a cog. Your bot is a cog, a web dashboard is a cog, even
caching is a cog.

This way of functioning is aided by the modularity offered by the microservice architecture
driven by using Kafka and provides several benefits, some of which are:

- Everything is opt-in so you can have only what you *need* running at once.
- Everything is extremely modular so you can mix, match, use and reuse cogs made
by you and other without any additional hassle.
- It's language agnostic which not only means you can use your preferred langauge
but you can also use different langauges for different parts of your bot or even
have a team of people who use different langauges work together on one thing.

...and much much more, the sky's the limit!

## Requirements

- Apache Kafka
- KeyDB
- Rust

## Usage

- Clone this repository via

```sh
git clone https://github.com/Eludris/cognite
```

- Change the variables in `.env.example` to work for you and rename it to `.env`
- Configure the `init.lua` file if you need to.
- Start a Kafka instance which has a topic maching the one in your `.env`
- Start cognite-gateway by running

```sh
cd cognite-gateway && cargo run --release
```

If you want to opt-in into caching must:

- Start a KeyDB instance.
- Start cognite-cache by running

```sh
cd cognite-cache && cargo run --release
```

Finally, start your cogs and you're all set!

## Official Wrappers
