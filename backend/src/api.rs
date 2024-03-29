use std::convert::identity;
use crate::challenge::{DbChallenge, ResChallenge};
use crate::commit::{Reaction, ReqCommit, ReqReaction, ResCommit};
use crate::executor;
use crate::executor::Language;
use crate::state::AppState;
use actix_identity::Identity;
use actix_web::cookie::time::macros::date;
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post, web, HttpResponse, Scope};
use actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use chrono::Utc;
use log::error;
use nom::complete::bool;
use nom::HexDisplay;
use rand::RngCore;
use sqlx::Error;

pub fn api_routes() -> Scope {
    web::scope("/api")
        .service(hello)
        .service(get_current_challenge)
        .service(submit_commit)
        .service(get_commit_by_id)
        .service(get_user)
        .service(current_user)
        .service(post_reaction)
        .service(get_commit_reactions)
        .service(get_user_commits)
        .service(get_commits)
        .service(get_commit_reactions_of_client)
        .service(get_latest_challenge_commits)
}

#[get("/")]
async fn hello() -> HttpResponse {
    HttpResponse::Ok().json("Hello from the GitReal Rust server 🚀!")
}

#[get("/challenge")]
async fn get_current_challenge(db: Data<AppState>, identity: Identity) -> HttpResponse {
    let user_id: i32 = match identity.id() {
        Ok(user_id) => user_id.parse().unwrap(),
        _ => {
            error!("User id not found");
            return HttpResponse::NotFound().body("User id not found.")
        },
    };

    // we need to get the user in order to set the default language
    let user = match db.get_user_by_id(user_id).await {
        Ok(user) => user,
        Err(err) => {
            error!("{err}");
            return HttpResponse::InternalServerError().body(err.to_string())
        },
    };

    let data = db.get_current_challenge().await;
    match data {
        Ok(challenge) => {
            let boilerplate =
                Language::for_all_languages(|l| challenge.function.generate_function(l));
            let res = ResChallenge {
                id: challenge.id,
                title: challenge.title,
                description: challenge.description,
                example_input: challenge.function.generate_example_input(),
                example_output: format!("{}", challenge.function.output),
                boilerplate,
                default_language: user.default_language,
                date_released: challenge.date_released,
                deadline: challenge.deadline,
            };
            HttpResponse::Ok().json(res)
        }
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[post("/commits")]
async fn submit_commit(
    db: Data<AppState>,
    new_commit: Json<ReqCommit>,
    identity: Identity,
) -> HttpResponse {
    let user_id = match identity.id() {
        Ok(user_id) => user_id.parse().unwrap(),
        _ => {
            error!("User id not found");
            return HttpResponse::NotFound().body("User id not found.")
        },
    };

    let challenge = match db.get_current_challenge().await {
        Ok(challenge) => challenge,
        Err(err) => {
            error!("{err}");
            return HttpResponse::InternalServerError().body(err.to_string())
        },
    };

    let (is_valid, _exec_result) = match executor::test_language(
        new_commit.language,
        challenge.function,
        new_commit.solution.as_str(),
    )
    .await
    {
        Ok(tuple) => tuple,
        Err(err) => {
            error!("{err}");
            return HttpResponse::InternalServerError().body(err.to_string())
        },
    };

    let mut data = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut data);

    let res = ResCommit {
        id: 0,
        commit_hash: hex::encode(data),
        user_id,
        date: Utc::now(),
        title: new_commit.title.clone(),
        solution: new_commit.solution.clone(),
        is_valid,
        language: new_commit.language,
        description: new_commit.description.clone(),
        challenge_id: challenge.id,
    };

    match db.add_commit(res).await {
        Ok(commit) => HttpResponse::Ok().json(commit),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[get("/commits")]
async fn get_commits(db: Data<AppState>, identity: Identity) -> HttpResponse {
    if identity.id().is_err() {
        return HttpResponse::NotFound().body("User id not found.");
    };

    match db.get_past_challenge_commits().await {
        Ok(commits) => HttpResponse::Ok().json(commits),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[get("/commits/{id}")]
async fn get_commit_by_id(db: Data<AppState>, commit_id: Path<i32>) -> HttpResponse {
    match db.get_commit_by_id(commit_id.into_inner()).await {
        Ok(commit) => HttpResponse::Ok().json(commit),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[get("/me")]
async fn current_user(db: Data<AppState>, identity: Identity) -> HttpResponse {
    let user_id = match identity.id() {
        Ok(user_id) => match user_id.parse() {
            Ok(id) => id,
            _ => {
                error!("User id is not a number");
                return HttpResponse::NotFound().body("User id is not a number.")
            }
        },
        _ => {
            error!("User id not found in the identity");
            return HttpResponse::NotFound().body("User id not found.")
        },
    };

    match db.get_me_info(user_id).await {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

// #[get("/challenges")]
// async fn get_challenges(db: Data<AppState>) -> HttpResponse {
//     match db.get_challenges().await {
//         Ok(challenges) => HttpResponse::Ok().json(challenges),
//         Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
//     }
// }

// #[get("/challenges/{id}/commits")]
// async fn get_past_challenge_commits(db: Data<AppState>, challenge_id: Path<i32>) -> HttpResponse {
//     match db.get_past_challenge_commits().await {
//         Ok(commits) => HttpResponse::Ok().json(commits),
//         Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
//     }
// }

#[get("/challenges/{id}")]
async fn get_past_challenge(db: Data<AppState>, challenge_id: Path<i32>) -> HttpResponse {
    match db.get_challenge_by_id(challenge_id.into_inner()).await {
        Ok(challenge) => HttpResponse::Ok().json(challenge),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[get("/challenge/commits")]
async fn get_latest_challenge_commits(db: Data<AppState>) -> HttpResponse {
    match db.get_past_challenge_commits().await {
        Ok(commits) => HttpResponse::Ok().json(commits),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}


#[get("/commits/{id}/reactions")]
async fn get_commit_reactions(
    db: Data<AppState>,
    // identity: Identity,
    challenge_id: Path<i32>,
) -> HttpResponse {
    // let user_id: i32 = match identity.id() {
    //     Ok(user_id) => user_id.parse().unwrap(),
    //     _ => return HttpResponse::NotFound().body("User id not found."),
    // };

    match db
        .get_commit_reactions(challenge_id.into_inner())
        .await
    {
        Ok(reaction_state) => HttpResponse::Ok().json(reaction_state),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[get("/commits/{id}/user-reactions")]
async fn get_commit_reactions_of_client(
    db: Data<AppState>,
    identity: Identity,
    challenge_id: Path<i32>
) -> HttpResponse {
    let user_id: i32 = match identity.id() {
        Ok(user_id) => match user_id.parse() {
            Ok(id) => id,
            Err(err) => {
                error!("Could not parse user id: {err}");
                return HttpResponse::InternalServerError().body(err.to_string())
            }
        },
        Err(err) => {
            error!("{err}");
            return HttpResponse::NotFound().body(err.to_string())
        },
    };

    match db
        .get_commit_reactions_by_user(user_id, challenge_id.into_inner())
        .await
    {
        Ok(user_reactions) => HttpResponse::Ok().json(user_reactions),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

#[get("/user/{id}")]
async fn get_user(db: Data<AppState>, user_id: Path<i64>) -> HttpResponse {
    match db.get_user(user_id.into_inner()).await {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[get("/user/{id}/commits")]
async fn get_user_commits(db: Data<AppState>, username: Path<i64>) -> HttpResponse {
    match db.get_commit_by_user_id(username.into_inner()).await {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}

#[post("/reactions")]
async fn post_reaction(db: Data<AppState>, identity: Identity, req_reaction: Json<ReqReaction>) -> HttpResponse {
    // Represents the user who reacts, not the author of the post.
    let user_id: i32 = match identity.id() {
        Ok(user_id) => match user_id.parse() {
            Ok(id) => id,
            Err(err) => {
                error!("{err}");
                return HttpResponse::InternalServerError().body(err.to_string())
            }
        },
        Err(err) => {
            error!("User id not found: {err}");
            return HttpResponse::NotFound().body(err.to_string())
        }
    };

    let reaction = Reaction {
        reaction_id: req_reaction.reaction_id,
        user_id,
        commit_id: req_reaction.commit_id,
        active: req_reaction.active
    };

    match db.post_reaction(reaction).await {
        Ok(reaction) => HttpResponse::Ok().json(reaction),
        Err(err) => {
            error!("{err}");
            HttpResponse::InternalServerError().body(err.to_string())
        },
    }
}
