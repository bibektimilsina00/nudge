//! The services that are a table.
//!
//! One entry per service, and adding one is adding an entry -- no code, and
//! nothing else in Nudge needs to know. See [`super::rest`] for why this is data.
//!
//! What goes here: services whose useful surface is a handful of plain REST
//! calls. What does not: anything needing pagination, streaming, file upload or
//! a response that has to be reshaped before it means anything. Those want a
//! real server, and pretending otherwise produces a connector that half works,
//! which is worse than one that is absent.
use super::rest::{Arg, Auth, Op, Put, Service};

const STR: &str = "string";
const NUM: &str = "integer";

/// Linear. Chosen first because its API is small, its token is one string, and
/// its scopes are visible on the page you make the token on.
const LINEAR_OPS: &[Op] = &[Op {
    name: "linear_me",
    about: "Who this token belongs to, and which teams they are in.",
    method: "POST",
    path: "/graphql",
    fixed: &[],
    args: &[Arg {
        name: "query",
        kind: STR,
        about: "GraphQL query. Start with `{ viewer { id name email } }`.",
        put: Put::Body,
        needed: true,
    }],
}];

pub const LINEAR: Service = Service {
    key: "linear",
    base: "https://api.linear.app",
    // Linear wants the token bare rather than as a Bearer, which is the sort of
    // thing that is only discoverable by trying it.
    auth: Auth::Header("Authorization"),
    headers: &[("Content-Type", "application/json")],
    ops: LINEAR_OPS,
};

/// Hunter, which is the simplest useful thing in the whole catalogue: one
/// endpoint, a key in the query string, and an answer.
const HUNTER_OPS: &[Op] = &[
    Op {
        name: "hunter_find_email",
        about: "Find the most likely email address for a person at a company.",
        method: "GET",
        path: "/v2/email-finder",
        fixed: &[],
        args: &[
            Arg {
                name: "domain",
                kind: STR,
                about: "Company domain, e.g. example.com",
                put: Put::Query,
                needed: true,
            },
            Arg {
                name: "first_name",
                kind: STR,
                about: "Their first name",
                put: Put::Query,
                needed: true,
            },
            Arg {
                name: "last_name",
                kind: STR,
                about: "Their last name",
                put: Put::Query,
                needed: true,
            },
        ],
    },
    Op {
        name: "hunter_verify_email",
        about: "Check whether an address exists and is deliverable.",
        method: "GET",
        path: "/v2/email-verifier",
        fixed: &[],
        args: &[Arg {
            name: "email",
            kind: STR,
            about: "The address to check",
            put: Put::Query,
            needed: true,
        }],
    },
    Op {
        name: "hunter_domain_search",
        about: "Addresses known at a domain.",
        method: "GET",
        path: "/v2/domain-search",
        fixed: &[],
        args: &[
            Arg {
                name: "domain",
                kind: STR,
                about: "Company domain",
                put: Put::Query,
                needed: true,
            },
            Arg {
                name: "limit",
                kind: NUM,
                about: "How many, up to 100",
                put: Put::Query,
                needed: false,
            },
        ],
    },
];

pub const HUNTER: Service = Service {
    key: "hunter",
    base: "https://api.hunter.io",
    auth: Auth::Query("api_key"),
    headers: &[],
    ops: HUNTER_OPS,
};

/// YouTube, signed in to rather than keyed.
///
/// An API key reaches public data and nothing else -- it cannot see your
/// channel, your private videos or your subscriptions, and cannot change
/// anything. That is the safer thing and the right default for "find me a
/// video". This is the other case: your own channel.
///
/// `youtube.readonly` and no more. It covers everything here, including private
/// videos and drafts, and stops short of upload, delete and comment -- which are
/// a different decision and should be made on purpose rather than inherited.
const YOUTUBE_OPS: &[Op] = &[
    Op {
        name: "youtube_my_channel",
        about: "Your own channel: title, description, subscriber and view counts.",
        method: "GET",
        path: "/youtube/v3/channels",
        fixed: &[
            ("part", "snippet,statistics,contentDetails"),
            ("mine", "true"),
        ],
        args: &[],
    },
    Op {
        name: "youtube_my_videos",
        about: "Your own videos, newest first, including private and unlisted ones.",
        method: "GET",
        path: "/youtube/v3/search",
        fixed: &[
            ("part", "snippet"),
            ("forMine", "true"),
            ("type", "video"),
            ("order", "date"),
        ],
        args: &[
            Arg {
                name: "q",
                kind: STR,
                about: "Words to match, or leave out for all",
                put: Put::Query,
                needed: false,
            },
            Arg {
                name: "maxResults",
                kind: NUM,
                about: "How many, up to 50",
                put: Put::Query,
                needed: false,
            },
        ],
    },
    Op {
        name: "youtube_my_playlists",
        about: "Your playlists.",
        method: "GET",
        path: "/youtube/v3/playlists",
        fixed: &[("part", "snippet,contentDetails"), ("mine", "true")],
        args: &[Arg {
            name: "maxResults",
            kind: NUM,
            about: "How many, up to 50",
            put: Put::Query,
            needed: false,
        }],
    },
    Op {
        name: "youtube_playlist_items",
        about: "The videos in one playlist.",
        method: "GET",
        path: "/youtube/v3/playlistItems",
        fixed: &[("part", "snippet,contentDetails")],
        args: &[
            Arg {
                name: "playlistId",
                kind: STR,
                about: "Which playlist",
                put: Put::Query,
                needed: true,
            },
            Arg {
                name: "maxResults",
                kind: NUM,
                about: "How many, up to 50",
                put: Put::Query,
                needed: false,
            },
        ],
    },
    Op {
        name: "youtube_my_subscriptions",
        about: "Channels you subscribe to.",
        method: "GET",
        path: "/youtube/v3/subscriptions",
        fixed: &[("part", "snippet"), ("mine", "true")],
        args: &[Arg {
            name: "maxResults",
            kind: NUM,
            about: "How many, up to 50",
            put: Put::Query,
            needed: false,
        }],
    },
    Op {
        name: "youtube_video_details",
        about: "Title, description, statistics and duration for one or more videos.",
        method: "GET",
        path: "/youtube/v3/videos",
        fixed: &[("part", "snippet,statistics,contentDetails")],
        args: &[Arg {
            name: "id",
            kind: STR,
            about: "Video id, or several separated by commas",
            put: Put::Query,
            needed: true,
        }],
    },
    Op {
        name: "youtube_search",
        about: "Search YouTube generally, not only your own videos.",
        method: "GET",
        path: "/youtube/v3/search",
        fixed: &[("part", "snippet"), ("type", "video")],
        args: &[
            Arg {
                name: "q",
                kind: STR,
                about: "What to search for",
                put: Put::Query,
                needed: true,
            },
            Arg {
                name: "maxResults",
                kind: NUM,
                about: "How many, up to 50",
                put: Put::Query,
                needed: false,
            },
        ],
    },
];

pub const YOUTUBE: Service = Service {
    key: "youtube",
    base: "https://www.googleapis.com",
    auth: Auth::Bearer,
    headers: &[],
    ops: YOUTUBE_OPS,
};

/// Google Tasks. A table rather than the npm server, which wants a client id, a
/// client secret and a refresh token as three separate variables -- an `Offer`
/// carries one, and the API underneath is four plain calls.
const TASKS_OPS: &[Op] = &[
    Op {
        name: "tasks_lists",
        about: "Your task lists.",
        method: "GET",
        path: "/tasks/v1/users/@me/lists",
        fixed: &[],
        args: &[],
    },
    Op {
        name: "tasks_in_list",
        about: "The tasks in one list.",
        method: "GET",
        path: "/tasks/v1/lists/{tasklist}/tasks",
        fixed: &[],
        args: &[
            Arg {
                name: "tasklist",
                kind: STR,
                about: "Which list, by id",
                put: Put::Path,
                needed: true,
            },
            Arg {
                name: "showCompleted",
                kind: "boolean",
                about: "Include finished ones",
                put: Put::Query,
                needed: false,
            },
            Arg {
                name: "maxResults",
                kind: NUM,
                about: "How many, up to 100",
                put: Put::Query,
                needed: false,
            },
        ],
    },
    Op {
        name: "tasks_add",
        about: "Add a task to a list.",
        method: "POST",
        path: "/tasks/v1/lists/{tasklist}/tasks",
        fixed: &[],
        args: &[
            Arg {
                name: "tasklist",
                kind: STR,
                about: "Which list, by id",
                put: Put::Path,
                needed: true,
            },
            Arg {
                name: "title",
                kind: STR,
                about: "What the task says",
                put: Put::Body,
                needed: true,
            },
            Arg {
                name: "notes",
                kind: STR,
                about: "Longer detail",
                put: Put::Body,
                needed: false,
            },
            Arg {
                name: "due",
                kind: STR,
                about: "RFC3339, e.g. 2026-09-20T00:00:00Z",
                put: Put::Body,
                needed: false,
            },
        ],
    },
    Op {
        name: "tasks_complete",
        about: "Mark a task finished.",
        method: "PATCH",
        path: "/tasks/v1/lists/{tasklist}/tasks/{task}",
        fixed: &[],
        args: &[
            Arg {
                name: "tasklist",
                kind: STR,
                about: "Which list, by id",
                put: Put::Path,
                needed: true,
            },
            Arg {
                name: "task",
                kind: STR,
                about: "Which task, by id",
                put: Put::Path,
                needed: true,
            },
            Arg {
                name: "status",
                kind: STR,
                about: "`completed` or `needsAction`",
                put: Put::Body,
                needed: true,
            },
        ],
    },
];

pub const TASKS: Service = Service {
    key: "google_tasks",
    base: "https://tasks.googleapis.com",
    auth: Auth::Bearer,
    headers: &[],
    ops: TASKS_OPS,
};

/// Search Console, signed in to rather than given a service account.
///
/// The npm server wants a service account key, which is a second kind of
/// credential to create, download and then separately grant on each property.
/// The same API answers an ordinary OAuth token, and `webmasters.readonly`
/// reaches exactly the properties you already own.
const GSC_OPS: &[Op] = &[
    Op {
        name: "gsc_sites",
        about: "The properties you have access to in Search Console.",
        method: "GET",
        path: "/webmasters/v3/sites",
        fixed: &[],
        args: &[],
    },
    Op {
        name: "gsc_performance",
        about: "Clicks, impressions and position, grouped however you ask.",
        method: "POST",
        path: "/webmasters/v3/sites/{siteUrl}/searchAnalytics/query",
        fixed: &[],
        args: &[
            Arg {
                name: "siteUrl",
                kind: STR,
                about: "e.g. https://example.com/ or sc-domain:example.com",
                put: Put::Path,
                needed: true,
            },
            Arg {
                name: "startDate",
                kind: STR,
                about: "YYYY-MM-DD",
                put: Put::Body,
                needed: true,
            },
            Arg {
                name: "endDate",
                kind: STR,
                about: "YYYY-MM-DD",
                put: Put::Body,
                needed: true,
            },
            Arg {
                name: "dimensions",
                kind: "array",
                about: "Any of query, page, country, device, date",
                put: Put::Body,
                needed: false,
            },
            Arg {
                name: "rowLimit",
                kind: NUM,
                about: "How many rows, up to 25000",
                put: Put::Body,
                needed: false,
            },
        ],
    },
    Op {
        name: "gsc_sitemaps",
        about: "Sitemaps submitted for a property, and what Google made of them.",
        method: "GET",
        path: "/webmasters/v3/sites/{siteUrl}/sitemaps",
        fixed: &[],
        args: &[Arg {
            name: "siteUrl",
            kind: STR,
            about: "The property",
            put: Put::Path,
            needed: true,
        }],
    },
];

pub const SEARCH_CONSOLE: Service = Service {
    key: "google_search_console",
    base: "https://searchconsole.googleapis.com",
    auth: Auth::Bearer,
    headers: &[],
    ops: GSC_OPS,
};

/// Everything declared here.
pub const ALL: &[Service] = &[LINEAR, HUNTER, YOUTUBE, TASKS, SEARCH_CONSOLE];

pub fn find(key: &str) -> Option<&'static Service> {
    ALL.iter().find(|s| s.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_has_a_unique_key_and_at_least_one_tool() {
        let mut keys: Vec<_> = ALL.iter().map(|s| s.key).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "two services share a key");
        for s in ALL {
            assert!(!s.ops.is_empty(), "{} offers nothing", s.key);
        }
    }

    #[test]
    fn every_tool_name_is_unique_across_the_catalogue() {
        // They land in one list in front of the model. Two `search` tools from
        // different services would be a coin flip.
        let mut names: Vec<_> = ALL
            .iter()
            .flat_map(|s| s.ops.iter().map(|o| o.name))
            .collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two tools share a name");
    }

    #[test]
    fn every_path_placeholder_has_an_argument_that_fills_it() {
        // A placeholder nobody fills makes a request with a literal `{id}` in
        // the URL, which comes back as somebody else's confusing 404.
        for s in ALL {
            for op in s.ops {
                for part in op.path.split('{').skip(1) {
                    let name = part.split('}').next().unwrap_or_default();
                    assert!(
                        op.args.iter().any(|a| a.name == name && a.put == Put::Path),
                        "{}/{} has {{{}}} in its path and no argument for it",
                        s.key,
                        op.name,
                        name
                    );
                }
            }
        }
    }

    #[test]
    fn every_argument_says_what_it_is_for() {
        for s in ALL {
            for op in s.ops {
                assert!(
                    !op.about.is_empty(),
                    "{}/{} has no description",
                    s.key,
                    op.name
                );
                for a in op.args {
                    assert!(
                        !a.about.is_empty(),
                        "{}/{}/{} has none",
                        s.key,
                        op.name,
                        a.name
                    );
                }
            }
        }
    }
}
