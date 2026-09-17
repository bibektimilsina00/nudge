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
const LINEAR_OPS: &[Op] = &[
    Op {
        name: "linear_me",
        about: "Who this token belongs to, and which teams they are in.",
        method: "POST",
        path: "/graphql",
        args: &[Arg {
            name: "query",
            kind: STR,
            about: "GraphQL query. Start with `{ viewer { id name email } }`.",
            put: Put::Body,
            needed: true,
        }],
    },
];

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
        args: &[
            Arg { name: "domain", kind: STR, about: "Company domain, e.g. example.com", put: Put::Query, needed: true },
            Arg { name: "first_name", kind: STR, about: "Their first name", put: Put::Query, needed: true },
            Arg { name: "last_name", kind: STR, about: "Their last name", put: Put::Query, needed: true },
        ],
    },
    Op {
        name: "hunter_verify_email",
        about: "Check whether an address exists and is deliverable.",
        method: "GET",
        path: "/v2/email-verifier",
        args: &[Arg { name: "email", kind: STR, about: "The address to check", put: Put::Query, needed: true }],
    },
    Op {
        name: "hunter_domain_search",
        about: "Addresses known at a domain.",
        method: "GET",
        path: "/v2/domain-search",
        args: &[
            Arg { name: "domain", kind: STR, about: "Company domain", put: Put::Query, needed: true },
            Arg { name: "limit", kind: NUM, about: "How many, up to 100", put: Put::Query, needed: false },
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

/// Everything declared here.
pub const ALL: &[Service] = &[LINEAR, HUNTER];

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
        let mut names: Vec<_> = ALL.iter().flat_map(|s| s.ops.iter().map(|o| o.name)).collect();
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
                        s.key, op.name, name
                    );
                }
            }
        }
    }

    #[test]
    fn every_argument_says_what_it_is_for() {
        for s in ALL {
            for op in s.ops {
                assert!(!op.about.is_empty(), "{}/{} has no description", s.key, op.name);
                for a in op.args {
                    assert!(!a.about.is_empty(), "{}/{}/{} has none", s.key, op.name, a.name);
                }
            }
        }
    }
}
