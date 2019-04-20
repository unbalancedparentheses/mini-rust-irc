- sacar enters
- hacer que haya word wrap
- que no imprima abajo en el input
- que no imprima arriba en la status
- status (number of windows)
- line editing abajo
- lectura y escritura del socket concurrente

- add TIME
- setlocale(LC_ALL, "");
- signal(SIGWINCH, sigwinch);
        
        // TODO send MODE  sndf("MODE %s +i", nick);
        // TODO user  sndf("USER %s 8 * :%s", user, user);


- manage reconnection
- emacs like keybinding
- terminal resize

nc irc.freenode.net 6667

PASS none
NICK sorandom29      
USER blah blah blah blah

PING :lindbohm.freenode.net
PONG :lindbohm.freenode.net

JOIN #linux

PRIVMSG #linux :hello guys! i'm using telnet to connect to irc and that's such a stupid idea, i have to respond to PINGs manually!

