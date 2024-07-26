package main

import (
	"log"
	"net/http"
	"wilkuu.xyz/yapnet/server"
)

func routes(s *yn_server.Server) {
	http.Handle("/ws", s)
	http.HandleFunc("/protocol.js", func(w http.ResponseWriter, r *http.Request) {
		s.Log.Logf("protocol path?: %v", r.URL.Path)
		http.ServeFile(w, r, "./protocol/protocol.js")
	})
	http.Handle("/", http.FileServer(http.Dir("./client")))

}

func main() {
	log.SetFlags(0)
	s := yn_server.NewServer(log.Printf)
	go s.Run()
	routes(s)
	log.Fatal(http.ListenAndServe(":8080", nil))
}
