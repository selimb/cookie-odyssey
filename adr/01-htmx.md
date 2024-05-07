# HTMX

I use Next.js at work.
Next.js is the backend, i.e. the backend is in Typescript.
However, HTMX is becoming very popular, and I want to see what all the fuss is about.

At first glance, I like the simplicity compared to Next.js: no bundling, no transpiling, no JS
churn, and no need to ship 1 million JS modules.
I can also play around with new (for me) technologies on the backend.

## Low Bandwidth

This is a travel blog application, and I want to be able to use it on mobile LTE.
Shipping megabytes upon megabytes of JS is not ideal.

One might argue that the JS bundle is typically cached after the first load, and that pushing
logic to the client can avoid roundtrips to the server.

## Hobby Project

This is a hobby project, and I want to be able to host this on a very cheap box, e.g. 5$/month
Digital Ocean droplet, with a database and everything.
In my experience, Node.js isn't known for its small memory/CPU usage.
HTMX lets me use a more efficient backend language, and mostly skip the frontend!
See [programming languages](02-language.md) for more info on the backend language.

## Type-Safety

I'm a bit worried that the stringly-typed nature of HTMX + server-side templates is going to drive
me crazy, especially coming from React+JSX+TRPC.
We'll see!
