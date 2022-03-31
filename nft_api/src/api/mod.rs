use hyper::{Body, Request, Response};
use routerify_json_response::{json_failed_resp_with_message, json_success_resp};

pub async fn handle_get_assets(
    req: Request<Body>,
) -> Result<Response<Body>, routerify_json_response::Error> {
    let users = ["Alice", "John"];
    json_success_resp(&users)
}
