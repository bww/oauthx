# OAuthX manages the OAuth2 authentication flow for CLIs
Sometimes you just need to get an OAuth token so you can plug it into a tool on the command line for some testing or one-off reason. _OAuth hates this and will stop at nothing to thwart you._

OAuthX, however, mostly shields you from OAuth's cruelty and makes this process sorta manageable. You provide OAuthX with the basic paramters for your OAuth consumer (which you will need to configure with the service in question) and it:

1. Launches your browser with the authorization URL to begin OAuth2 flow,
2. Starts a local web server to receive the callback redirect from the service,
3. Exchanges the token once it receives that callback, and
4. Prints the token as JSON so that a CLI tool can use it; and also displays the token in the browser.

Pretty good, right?

Externally, you will need to:

1. Configure an OAuth consumer with your service and get the relevant parameters, and
2. Deal with getting some sort of proxy to forward the callback URL from the internet to the OAuthX local service.

So, you know... it is what it is. Good luck!
