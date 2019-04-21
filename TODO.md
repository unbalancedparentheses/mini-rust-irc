* TODO 

- remove rustyline and use tuirs
- resolve bugs related to implementing a status bar and a prompt (bar on top is being printed really slow)
- if enter is pressed while in a channel, nothing should be sent
- set a global state (with mutex since it is going to be touched by many threads) with the channel we are connected to. implement a better app state 
- implement prompt (irc should not write at the last line of the shell)
- implement status bar using multiple windows
- add option parsing that overwrites the for connecting to server with a particular user and to a list of channels
- don't print the name of the server each time
- wait until message of quit is sent before exiting the main process
- implement fmd display for irc message
- from string should return a result since it can fail
- manage registration timeout when the nick is already in use or other uses cases
- check if the match cmd can be implemented in a better way instead of using vec<&str>
- manage reconnection
- support  MODE
- add ssl
- remove and solve todos from codebase

* Links to read

https://aml3.github.io/RustTutorial/

https://channel9.msdn.com/Blogs/Seth-Juarez/Anders-Hejlsberg-on-Modern-Compiler-Construction

https://manishearth.github.io/blog/2015/05/27/wrapper-types-in-rust-choosing-your-guarantees/

https://github.com/Nervengift/chat-server-example/blob/master/src/main.rs

https://nbaksalyar.github.io/2015/07/10/writing-chat-in-rust.html

https://jsdw.me/posts/rust-asyncawait-preview/

https://stackoverflow.com/questions/33116317/when-would-you-use-a-mutex-without-an-arc
