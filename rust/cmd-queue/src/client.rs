use url::Url;

use crate::{error::CmdqClientError, CommandRequest, CommandResponse, Task, TaskState};

pub struct Client {
    client: reqwest::blocking::Client,
    host: Url,
}

impl Client {
    pub fn new(host: &str) -> Result<Self, CmdqClientError> {
        let client = reqwest::blocking::Client::new();
        let host = Url::parse(host)
            .map_err(|e| CmdqClientError::ServerHostUrlParseError(host.to_string(), e))?;
        Ok(Client { client, host })
    }

    pub fn queue_command(
        &self,
        cmd_req: CommandRequest,
    ) -> Result<CommandResponse, CmdqClientError> {
        let mut req_url = self.host.clone();
        req_url.set_path("api/commands");

        let response = self
            .client
            .post(req_url)
            .json(&cmd_req)
            .send()
            .map_err(|e| CmdqClientError::HttpClientError(e))?;

        //println!("{:?}", response);
        let cmd_response = response
            .json::<CommandResponse>()
            .map_err(|e| CmdqClientError::ResponseDeserializationError(e))?;
        Ok(cmd_response)
    }

    pub fn list_tasks(&self, state_filters: Vec<TaskState>) -> Result<Vec<Task>, CmdqClientError> {
        todo!("deprecated. To remove")
    }
}
