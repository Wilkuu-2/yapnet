# Yapnet
A flexible protocol and server-client implementation for text based games.

> [!WARNING]
> This project is in it's prototype stages and lacks most of the features.
> Do not use it in anything you expect to actually work!

## Protocol
The protocol is meant to be simple, but extensible.
The server maintains a list of protocol messages as state. 
This means that a proper implementation could just pop messages from this list to go back in time. 
The current implementation uses websockets for communication and will support pulling assets over Http using predefined api. 

## Implementation 
The implementation of the server and client are written in Rust and will support writing the game logic in Lua.



