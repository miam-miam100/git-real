use crate::auth::{MeInfo, UserInfo};
use crate::challenge::DbChallenge;
use crate::commit::{Reaction, ReactionState, ReqCommit, ResCommit, UserReactions};
use chrono::Utc;
use log::info;
use oauth2::basic::BasicClient;
use sqlx::error::Error;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub oauth: BasicClient,
    pub db: Pool<Postgres>,
}

impl AppState {
    pub fn new(oauth: BasicClient, db: Pool<Postgres>) -> Self {
        AppState { oauth, db }
    }

    pub async fn get_current_challenge(&self) -> Result<DbChallenge, Error> {
        let result: DbChallenge = sqlx::query_as!(
            DbChallenge,
            "SELECT * FROM public.challenges WHERE date_released <= $1 ORDER BY deadline DESC LIMIT 1",
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(result)
    }

    pub async fn get_commit_by_id(&self, commit_id: i32) -> Result<ResCommit, Error> {
        let result: ResCommit = sqlx::query_as!(
            ResCommit,
            "SELECT * FROM public.commits WHERE id = $1 ORDER BY date DESC",
            commit_id
        )
        .fetch_one(&self.db)
        .await?;

        Ok(result)
    }

    pub async fn get_user(&self, user_id: i64) -> Result<UserInfo, Error> {
        let result: UserInfo =
            sqlx::query_as!(UserInfo, "SELECT * FROM public.users WHERE id=$1", user_id)
                .fetch_one(&self.db)
                .await?;

        Ok(result)
    }

    pub async fn get_commit_by_user_id(&self, user_id: i64) -> Result<Vec<ResCommit>, Error> {
        let result = sqlx::query_as!(
            ResCommit,
            "SELECT * FROM public.commits WHERE user_id=$1 AND is_valid='true' ORDER BY date DESC",
            user_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(result)
    }

    pub async fn add_or_update_user(&self, user: &UserInfo) -> anyhow::Result<bool> {
        let res = sqlx::query!(
            "INSERT INTO users (id, name, username, avatar_url) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING",
            user.id,
            user.name,
            user.username,
            user.avatar_url
        )
            .execute(&self.db)
            .await?;

        Ok(res.rows_affected() > 0)
    }

    pub async fn get_user_by_id(&self, user_id: i32) -> anyhow::Result<UserInfo> {
        let user = sqlx::query_as!(
            UserInfo,
            "SELECT * FROM users WHERE id = $1",
            user_id as i32
        )
        .fetch_one(&self.db)
        .await?;
        Ok(user)
    }

    pub async fn get_me_info(&self, user_id: i64) -> anyhow::Result<MeInfo> {
        // info!("User id is {user_id}");
        let user =
            sqlx::query_as::<_, UserInfo>(&format!("SELECT * FROM users WHERE id = {user_id}"))
                .fetch_one(&self.db)
                .await?;
        // let user = sqlx::query_as!(UserInfo, "SELECT * FROM users WHERE id = $1", user_id)
        //     .fetch_one(&self.db)
        //     .await?;
        // info!("User is {:?}", user);
        let record = sqlx::query!(
            r#"
            SELECT is_valid
            FROM commits
            WHERE user_id = $1 AND is_valid = 'true'
            ORDER by date DESC LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        // info!("Record is {:?}", record);

        Ok(MeInfo {
            id: user.id,
            name: user.name,
            username: user.username,
            avatar_url: user.avatar_url,
            default_language: user.default_language,
            completed_correctly: record.is_some(),
        })
    }

    pub async fn add_commit(&self, commit: ResCommit) -> Result<ResCommit, Error> {
        let result = sqlx::query!(
            r#"
            INSERT INTO commits (commit_hash, user_id, date, title, solution, is_valid, language, description, challenge_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
            "#,
            commit.commit_hash,
            commit.user_id,
            commit.date,
            commit.title,
            commit.solution,
            commit.is_valid,
            commit.language as i32,
            commit.description,
            commit.challenge_id
        )
            .fetch_one(&self.db)
            .await?;

        self.get_commit_by_id(result.id).await
    }

    pub async fn get_challenge_by_id(&self, id: i32) -> Result<DbChallenge, Error> {
        let result: DbChallenge = sqlx::query_as!(
            DbChallenge,
            "SELECT * FROM public.challenges WHERE id=$1",
            id
        )
        .fetch_one(&self.db)
        .await?;

        Ok(result)
    }

    pub async fn get_past_challenges(&self) -> Result<Vec<DbChallenge>, Error> {
        let result = sqlx::query_as!(
            DbChallenge,
            "SELECT * FROM public.challenges ORDER BY deadline DESC LIMIT 10"
        )
        .fetch_all(&self.db)
        .await?;
        Ok(result)
    }

    pub async fn get_past_challenge_commits(&self) -> Result<Vec<ResCommit>, Error> {
        let result = sqlx::query_as!(
            ResCommit,
            "SELECT * FROM public.commits WHERE is_valid = 'true' ORDER BY date DESC",
        )
        .fetch_all(&self.db)
        .await?;
        Ok(result)
    }

    pub async fn get_commit_reactions(
        &self,
        commit_id: i32,
    ) -> Result<ReactionState, Error> {
        let mut vec: Vec<i32> = vec![];

        for reaction_id in 0..9 {
            let reaction_count = sqlx::query!(
                r#"
                SELECT COUNT(*) FROM user_reactions
                WHERE commit_id=$1 AND reaction_id=$2 AND active=true
                "#,
                commit_id,
                reaction_id
            )
            .fetch_one(&self.db)
            .await?;

            vec.push(reaction_count.count.unwrap_or(0) as i32);
        }

        let reaction_state = ReactionState {
            heart: vec[0],
            rocket: vec[1],
            thumbsup: vec[2],
            thumbsdown: vec[3],
            skull: vec[4],
            trash: vec[5],
            tada: vec[6],
            facepalm: vec[7],
            nerd: vec[8],
        };

        // info!("Reaction state is {:?}", reaction_state);

        Ok(reaction_state)
    }

    pub async fn get_commit_reactions_by_user(
        &self,
        user_id: i32,
        commit_id: i32
    ) -> Result<UserReactions, Error> {
        let mut vec: Vec<bool> = vec![];

        for reaction_id in 0..9 {
            let active_record = sqlx::query!(
                r#"
                SELECT active FROM user_reactions
                WHERE commit_id=$1 AND reaction_id=$2 AND user_id=$3
                "#,
                commit_id,
                reaction_id,
                user_id
            )
                .fetch_optional(&self.db)
                .await?;

            let active= match active_record {
                Some(r) => r.active,
                None => false
            };

            vec.push(active);
        }

        let user_reactions = UserReactions {
            heart: vec[0],
            rocket: vec[1],
            thumbsup: vec[2],
            thumbsdown: vec[3],
            skull: vec[4],
            trash: vec[5],
            tada: vec[6],
            facepalm: vec[7],
            nerd: vec[8],
        };

        Ok(user_reactions)
    }

    pub async fn post_reaction(&self, incoming: Reaction) -> Result<ReactionState, Error> {
        sqlx::query!(
            r#"
            INSERT INTO user_reactions (reaction_id, user_id, commit_id, active)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (reaction_id, user_id, commit_id)
            DO UPDATE SET active = $4
            "#,
            incoming.reaction_id,
            incoming.user_id,
            incoming.commit_id,
            incoming.active,
        )
            .execute(&self.db)
            .await?;

        self.get_commit_reactions(incoming.commit_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn post_reaction_test() {

    }
}
