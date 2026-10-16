# Security policy

glance runs git and reads its output; it never sends anything over the network itself (the background `git fetch`
is git's own).

If you find a vulnerability — for instance, crafted repository content or git output that makes glance run
something, write outside its config file, or emit terminal escape sequences it should have stripped — please
**do not open a public issue**. Report it privately through
[GitHub's private vulnerability reporting](https://github.com/sachadvr/glance/security/advisories/new).

You will get an answer within a week. Fixes are released as soon as they are ready, and credited to you unless you
prefer otherwise.

Only the latest release is supported.
