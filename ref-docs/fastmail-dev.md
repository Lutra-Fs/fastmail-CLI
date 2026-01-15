Title: API Documentation

URL Source: http://www.fastmail.com/dev/

Markdown Content:
Welcome, developers!
--------------------

**Looking to integrate with Fastmail? You’ve come to the right place.**

Fastmail supports open standards. We provide full support for access to the data in your Fastmail account over the following open protocols:

*   Mail — you can access mail via [JMAP](https://www.rfc-editor.org/rfc/rfc8621.html), [IMAP](https://www.ietf.org/rfc/rfc9051.html), or [POP](https://www.rfc-editor.org/rfc/rfc1939) and send via [JMAP](https://www.rfc-editor.org/rfc/rfc8621.html) or [SMTP](https://www.rfc-editor.org/rfc/rfc5321.html).
*   Contacts — you can access via [CardDAV](https://www.rfc-editor.org/rfc/rfc6352). We will be opening up JMAP access as well, as soon as the specification is finalized.
*   Calendars — you can access via [CalDAV](https://www.rfc-editor.org/rfc/rfc4791). We will be opening up JMAP access as well, as soon as the specification is finalized.
*   Files — you can access via [WebDAV](https://www.rfc-editor.org/rfc/rfc4918).

The full API specification documents are linked above, and there are many libraries, tutorials, and other resources on the internet for building with these open protocols, so we won’t repeat it all here.

The most important thing to note are the authentication requirements. If you are building an app to distribute to others, you should implement OAuth to allow users to easily and securely authenticate. You can use OAuth with all of our supported protocols; please see our guide below for more information on integrating OAuth with Fastmail.

For testing purposes, or building an app for just yourself, you can generate an API token (for JMAP access), or an app password (for everything else) in [Settings → Privacy & Security → Integrations](https://app.fastmail.com/settings/security/integrations).

Getting started with JMAP
-------------------------

JMAP is a modern protocol that provides standard methods for efficiently synchronising data with a server. It lets us provide a consistent interface to all the different data types in a Fastmail account, such as emails, calendar events, or contacts.

Like most modern APIs, JMAP uses HTTP and JSON so it’s easy to implement in just about any language. We’ve written a short [JMAP crash course](https://jmap.io/crash-course.html), and published open source [sample code](https://github.com/fastmail/JMAP-Samples) to our GitHub account to help you get started.

To start using a JMAP API you need two things: the URL of the JMAP Session resource and authentication credentials. The JMAP session resource can be found at `https://api.fastmail.com/jmap/session`. To authenticate, you’ll need an API token, as discussed above.

To use an API token, add an`Authorization`header set to`Bearer {value}`, where`{value}`is the value of the token to your API request. Our JMAP APIs are CORS enabled so you can create integrations directly from a web app, as well as from a server or native code.

Masked Email API
----------------

We support a JMAP extension to allow apps to manage a user’s Masked Email addresses. The `https://www.fastmail.com/dev/maskedemail` capability is advertised to represent support for the MaskedEmail data type and associated methods.

Each MaskedEmail object contains a number of metadata fields:`createdBy`, `forDomain`, `url` and `description`. Make sure you set these correctly so the Fastmail app and other products all interoperate.

The`createdBy`property will be set by the server to the name of the client that created this masked email address. This depends on the authentication method used (OAuth client, API token, web interface, etc). Your product should**not**try to set this.

_If_ your product is a password manager, the`forDomain`property should be the protocol and domain (i.e. origin) of the site the user is using the masked email for. It should**not**include any path or other components. Example:`https://www.example.com`

_If_ your product supports deep links, the`url`property should be a deep link to the credential or other record related to this masked email address in your product. If deep links are not supported, leave this`null`. Do**not**set this to the domain of the site the user is using the masked email for, see above and use`forDomain`for this.

The`description`property should only be set to a short user supplied description of what this masked email address is for. If the user did not supply a description, leave it as the empty string. Do**not**set this to the name of your product, see`createdBy`above which is used for this. Do**not**set this to the domain of the site the user is using the masked email for, see above and use`forDomain`for this.

### The MaskedEmail data type

This capability adds support for a new MaskedEmail data type, with the following properties:

*   **id**: `String` (immutable; server-set)

 The id of the masked email address.
*   **email**: `String` (immutable; server-set)

 The email address.
*   **state**: `String` (default: pending)

 This MUST be one of: - `pending`: the initial state. Once set to anything else, it cannot be set back to pending. If a message is received by an address in the “pending” state, it will automatically be converted to “enabled”. Pending email addresses are automatically deleted 24h after creation. - `enabled`: the address is active and receiving mail normally. - `disabled`: the address is active, but mail is sent straight to trash. - `deleted`: the address is inactive; any mail sent to the address is bounced.
*   **forDomain**: `String`

 The domain name of the site this address was created for, e.g. “https://example.com”. This is intended to be added automatically by password managers.
*   **description**: `String`

 A short description of what this email address is for. This will be displayed next to the email to help users identify it, and also be used to tag mail sent to it.
*   **lastMessageAt**: `UTCDate|null` (server-set)

 The date-time the most recent message was received at this email address, if any.
*   **createdAt**: `UTCDate` (immutable; server-set)

 The date-time the email address was created.
*   **createdBy**: `String` (immutable; server-set)

 The name of the client that created this email address. This will be set by the server automatically based on the credentials used to authenticate the request, e.g. “ACME Password Manager”.
*   **url**: `String|null`

 A URL pointing back to the integrator’s use of this email address, e.g. a custom-uri to open “ACME Password Manager” at the appropriate entry.
*   **emailPrefix**: `String` (create-only)

 This is only used on create and otherwise ignored; it is not returned when MaskedEmail objects are fetched. If supplied, the server-assigned email will start with the given prefix. The string MUST be <= 64 characters in length and MUST only contain characters a-z, 0-9 and _ (underscore).

There are two methods added by the capability

### MaskedEmail/get

This is a standard “/get” method as described in [RFC8620, Section 5.1](https://www.rfc-editor.org/rfc/rfc8620.html#section-5.1). The “ids” argument may be `null` to fetch all at once.

### MaskedEmail/set

This is a standard “/set” method as described in [RFC8620, Section 5.3](https://www.rfc-editor.org/rfc/rfc8620.html#section-5.3).

The email address will be generated by the server as follows:

1.   The domain will be set as per the Masked Email domain setting for the customer to which the user belongs.
2.   The mailbox will start with the given “emailPrefix” if provided, otherwise, the server will generate a random one from a predefined corpus.

To prevent abuse, there are rate limits in place. A standard JMAP`rateLimit`SetError will be returned on create if these are exceeded.

OAuth at Fastmail
-----------------

Fastmail can authorize third-party clients to access a user’s data using OAuth 2.0, in conformance with the Authorization Code grant flow of the OAuth 2.0 Authorization Framework, as defined in[RFC 6749](https://www.rfc-editor.org/rfc/rfc6749.html).

### Scopes

The following scopes are currently supported:

**urn:ietf:params:jmap:core**This grants access to use the JMAP protocol and fetch the user’s session object. You MUST request this scope if requesting any other JMAP-based scope.**urn:ietf:params:jmap:mail**This represents support for the Mailbox, Thread, Email, and SearchSnippet data types and associated API methods. This is the scope you need to read and manage mail for a user. Full documentation is in [RFC8621](https://www.rfc-editor.org/rfc/rfc8621.html#section-1.3.1).**urn:ietf:params:jmap:submission**This represents support for the Identity and EmailSubmission data types and associated API methods. You will need this scope if you want to send mail for a user via JMAP. Full documentation is in [RFC8621](https://www.rfc-editor.org/rfc/rfc8621.html#section-1.3.1).**urn:ietf:params:jmap:vacationresponse**This represents support for the VacationResponse data type and associated API methods. With this scope, you can change the user’s vacation response and turn it on or off.**https://www.fastmail.com/dev/maskedemail**This grants the ability to read and write the MaskedEmail data type, as documented above.
Registration
------------

Clients are currently registered manually by contact with Fastmail developers. Want to connect your app with OAuth?[Check out our partnerships page and get in touch!](https://www.fastmail.com/company/partners/)

We’ll need the following information to register your client, but we can update this later, so don’t worry if you’re starting something new and don’t have everything to hand yet — we’d still love to hear from you.

*   **clientName**: The name of your app, to be shown to the user during authorization.
*   **logoUrl**: The URL for a logo to display for your app. This will be rendered in a square shape.
*   **clientUrl**: The URL of a web page providing information about your app (e.g. the page on the App store, your website, or a GitHub link).
*   **tosUrl**: The URL for your terms of service.
*   **policyUrl**: The URL for your privacy policy.
*   **supportUrl**: The URL of a web page that tells users how to contact you for support with your app.
*   **scopes:** The list of scopes your app may request access for. This should be the minimal set of scopes needed for your app to function. For example, if you are building a password manager that just manages the user’s Masked Email addresses, you should not request the `urn:ietf:params:jmap:mail` scope, which would give access to the messages the user receives, as the app has no need for this.
*   **redirectUris**: A list of redirect URIs the client may use in the OAuth flow. You can register as many as you like. Each redirect URI: 
    *   MUST be a full URI including path if desired.
    *   MAY include query parameters.
    *   MUST NOT include a fragment part (`#`).
    *   MUST NOT include path traversal (`[\/]..`).
    *   MUST start with one of: 
        *   An https domain you own, e.g.`https://example.com/`(This is referred to as a “claimed https URI” in OAuth docs.)
        *   A private-use URI scheme in reverse domain notation, e.g.`com.example:/`. Such a scheme MUST have at least one dot in it. It SHOULD be formed from a domain you own.
        *   `http://localhost/`– the loopback URL. During authorization, an arbitrary port may be given by the client, and`localhost`may be replaced by either`127.0.0.1`or`::1`.

*   **clientOrigins**: A list of origins the client may use to make [CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS) requests to the token endpoint and JMAP API. If your app is a web app and connecting directly to our API (rather than via your servers), you’ll need to ensure you register the origins you will be using for it to work. If you are not building a web-based app, you don’t need this.

We will assign a client id for the registration and send it back to you.

If the client supports OAuth with multiple authorization servers (e.g. it adds support for another email service) it MUST NOT reuse the same redirect URI with different registrations. (For example, it should add a per-service path component.) This allows it to detect and prevent server mix-up attacks (see the state parameter below).

Clients initiate authorization by opening the user’s web browser at the Fastmail OAuth Authorization URL:

`https://api.fastmail.com/oauth/authorize`

The following URL parameters MUST be present:

*   **client_id**: The client id for the application, given to the developer when your application is registered.
*   **redirect_uri**: One of the redirect URIs previously registered with Fastmail. This MUST be identical to the registered URI. The one exception to this is if a URI with the prefix `http://localhost/` was registered, in which case the matching URI MUST also include an arbitrary port and MAY use 127.0.0.1 or ::1 instead of `localhost`.

For example, if`http://localhost/redirect`was registered, then the client could send`http://127.0.0.1:49152/redirect`as the redirect_uri for authorization.

*   **response_type**: This MUST be “code”. No other value is accepted at this time.
*   **scope**: A space delimited set of scopes the client would like access to. This MUST be a subset of the scopes registered for the client.
*   **code_challenge**: A PKCE code challenge as per[RFC 7636](https://www.rfc-editor.org/rfc/rfc7636.html), using SHA-256. To generate a challenge, first generate a “code_verifier”: a high-entropy cryptographic random string using the characters`[A-Z] / [a-z] / [0-9] / "-" / "." / "*" / "~"`(the unreserved characters from Section 2.3 of[RFC 3986](https://www.rfc-editor.org/rfc/rfc3986.html)), with a minimum length of 43 characters and a maximum length of 128 characters. The code_challenge is then`BASE64URL-ENCODE(SHA256(ASCII(code_verifier)))`.
*   **code_challenge_method**: The only accepted value at this time is “S256”.
*   **state**: An opaque value used by the client to verify that an authorization response is due to a request that the client initiated. The Fastmail server will include this value when redirecting the user-agent back to the client. Clients MUST generate a state with a unique, unguessable random string when initiating an authorization request. When a response is received, the client MUST: - Verify that the state returned matches exactly the state it sent, to verify that this request was indeed initiated by the client and not an attacker. - Verify that the redirect URI the authorization response came in on was the one they used when generating the request. This prevents Mix-Up Attacks when the client supports more than one authorization server.

Other URL parameters MAY be supplied but will be ignored.

Fastmail will authenticate the user and ask them if they wish to grant authorization to the client. If successful, the client will receive a response via the redirect_uri, which will include the following query parameters:

*   **code**: The authorization code. This may be exchanged for the refresh token. It expires 10 minutes after authorization. It MUST NOT be used again once the client has successfully exchanged it for a refresh token. This will result in all authentication tokens associated with it being immediately revoked.
*   **state**: The value of the state parameter that was passed in with the initial request.

If the authorization fails due to a missing or invalid client_id or redirect_uri parameter, the user agent will not redirect back to the client. Otherwise, the client will receive a response via the redirect_uri, which will include the following query parameters:

*   **error**: An error type, as per[RFC 6749, section 4.1.2.1](https://www.rfc-editor.org/rfc/rfc6749.html#section-4.1.2.1).
*   **state**: The value of the state parameter that was passed in with the initial request.

### Obtaining a refresh token

Following authorization, the client will obtain initial refresh tokens and access tokens by making a POST request to the Fastmail token endpoint:

`https://api.fastmail.com/oauth/refresh`

The following parameters MUST be present, using the “application/x-www-form-urlencoded” format with a character encoding of UTF-8 in the HTTP request entity-body:

*   **client_id**: The client id for the application, given to the developer when your application is registered.
*   **redirect_uri**: The redirect_uri parameter sent with the authorization request from which the code was obtained.
*   **grant_type**: This MUST be “authorization_code”.
*   **code**: The code returned via the redirect back from authorization.
*   **code_verifier**: The code_verifier generated for the authorization, as per[RFC 7636](https://www.rfc-editor.org/rfc/rfc7636.html).

Other parameters MAY be supplied but will be ignored.

The server will verify the parameters and if successful, return a`200 OK`response with a content type of `application/json`. The body will be a JSON object with the following properties:

*   **access_token**:`String`A bearer token used to authenticate API requests. This will be valid for a fixed, limited time.
*   **token_type**:`String`The type of the access token. For now, this will always be “bearer”.
*   **expires_in**:`Number`The lifetime in seconds of the access token. For example, the value`3600`denotes that the access token will expire in one hour from the time the response was generated.
*   **scope**:`String`The space delimited set of scopes that this access token may use.
*   **refresh_token**:`String`The refresh token to use next time the client needs to get a new access token.

If the request fails, the server will return an appropriate HTTP error response, probably`400 Bad Request`, with a content type of `application/json`. The body will be a JSON object with the following property:

*   **error**:`String`A single ASCII error code indicating why the request failed, with a value as per[RFC 6749, Section 5.2](https://www.rfc-editor.org/rfc/rfc6749.html#section-5.2).

### Getting a new access token

Your client should keep using the access token it has been issued until the token expires, which will result in getting a`401`HTTP response back.

When the access token expires, the client must get a new one by making a POST request to the Fastmail token endpoint again. The following parameters MUST be present, using the “application/x-www-form-urlencoded” format with a character encoding of UTF-8 in the HTTP request entity-body:

*   **client_id**: The client id for the application, given to the developer when your application is registered.
*   **grant_type**: This MUST be “refresh_token”.
*   **refresh_token**: The refresh token returned last time the client obtained a new access token.

The success and failures responses are identical to those documented in the “obtaining a refresh token” section above.

In the case of an “invalid_grant” error, there MAY be an additional property called “temporary”. If this is present and the value is true, the refresh token MAY become valid again at some point in the future. (This can happen if a user is locked by their admin, but then unlocked again at a later point.) The “error_description” property of the error object SHOULD be present with a user-readable explanation that the client may like to show. If the “temporary” property is not present, an invalid_grant error is permanent and the user will need to go through the OAuth flow again to get a new refresh token if they wish to reconnect to their Fastmail account.

**Please note**, a new refresh token will be returned in the response and the client MUST replace their previous refresh token with this. The client MUST NOT try to use an old refresh token again; this will result in the authorization being revoked as a protection against leaked refresh tokens. If the user has multiple devices, each client MUST obtain separate authorization. You cannot share a refresh token between devices.

### Revoking a token

If a client no longer needs the access it has been granted to an account, it can make an authenticated POST request to the following URL to immediately revoke all associated access tokens and refresh tokens:

`https://api.fastmail.com/oauth/revoke`