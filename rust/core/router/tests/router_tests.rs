//! Router Tests
//!
//! Unit tests for the router module.

use router::{
    RouterImpl, HandleInputRequest, ParsedInput, RouterResponse,
    CommandType, CommandInfo, UserCommand, CommandHandler,
    CommandHandlerType, CommandVariable,
};

#[tokio::test]
async fn test_router_initialization() {
    let router = RouterImpl::new();
    assert!(!router.is_initialized());

    router.initialize().await.unwrap();
    assert!(router.is_initialized());
}

#[tokio::test]
async fn test_handle_input_non_command() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    let result = router
        .handle_input(HandleInputRequest {
            input: "Hello world".to_string(),
            session_id: "test".to_string(),
        })
        .await;

    assert!(result.to_agent);
}

#[tokio::test]
async fn test_handle_input_builtin_command() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    let result = router
        .handle_input(HandleInputRequest {
            input: "/help".to_string(),
            session_id: "test".to_string(),
        })
        .await;

    assert!(!result.to_agent);
    assert!(result.response.success);
}

#[tokio::test]
async fn test_handle_input_unknown_command() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    let result = router
        .handle_input(HandleInputRequest {
            input: "/unknowncmd".to_string(),
            session_id: "test".to_string(),
        })
        .await;

    assert!(!result.to_agent);
    assert!(!result.response.success);
}

#[tokio::test]
async fn test_handle_input_empty() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    let result = router
        .handle_input(HandleInputRequest {
            input: "".to_string(),
            session_id: "test".to_string(),
        })
        .await;

    assert!(!result.to_agent);
    assert!(!result.response.success);
}

#[tokio::test]
async fn test_list_commands() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    let commands = router.list_commands(None).await;
    assert!(!commands.is_empty());

    let builtin_only = router.list_commands(Some("builtin")).await;
    assert!(builtin_only.iter().all(|c| c.command_type == CommandType::Builtin));
}

#[tokio::test]
async fn test_help_with_args() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    let result = router
        .handle_input(HandleInputRequest {
            input: "/help status".to_string(),
            session_id: "test".to_string(),
        })
        .await;

    assert!(!result.to_agent);
    assert!(result.response.success);
}

#[tokio::test]
async fn test_is_command() {
    assert!(RouterImpl::is_command("/help"));
    assert!(RouterImpl::is_command("/cmd arg"));
    assert!(!RouterImpl::is_command("Hello"));
    assert!(!RouterImpl::is_command(""));
}

#[tokio::test]
async fn test_command_aliases() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    // Test '?' alias for help
    let result = router
        .handle_input(HandleInputRequest {
            input: "/?".to_string(),
            session_id: "test".to_string(),
        })
        .await;

    assert!(result.response.success);
}

#[tokio::test]
async fn test_register_user_command() {
    let router = RouterImpl::new();
    router.initialize().await.unwrap();

    let user_cmd = UserCommand {
        name: "greet".to_string(),
        description: "Greet someone".to_string(),
        template: "Hello, {{name}}!".to_string(),
        variables: vec![CommandVariable {
            name: "name".to_string(),
            description: "Name to greet".to_string(),
            required: true,
            default: None,
        }],
        handler: CommandHandler {
            handler_type: CommandHandlerType::CommandModule,
            name: "greet".to_string(),
        },
    };

    router.register_user_command(user_cmd).await.unwrap();

    let result = router
        .handle_input(HandleInputRequest {
            input: "/greet".to_string(),
            session_id: "test".to_string(),
        })
        .await;

    assert!(result.response.success);
}

#[tokio::test]
async fn test_parsed_input_command() {
    let input = ParsedInput::new("/help search");
    assert!(input.is_command);
    assert_eq!(input.command_name, Some("help".to_string()));
    assert_eq!(input.args, vec!["search"]);
}

#[tokio::test]
async fn test_parsed_input_agent_input() {
    let input = ParsedInput::new("How are you?");
    assert!(!input.is_command);
    assert!(input.command_name.is_none());
}

#[tokio::test]
async fn test_parsed_input_empty() {
    let input = ParsedInput::new("");
    assert!(!input.is_command);
    assert!(input.is_empty());
}

#[tokio::test]
async fn test_router_response_success() {
    let response = RouterResponse::success("Done");
    assert!(response.success);
    assert_eq!(response.message, "Done");
    assert!(!response.to_agent);
}

#[tokio::test]
async fn test_router_response_error() {
    let response = RouterResponse::error("Failed");
    assert!(!response.success);
    assert_eq!(response.error, Some("Failed".to_string()));
}

#[tokio::test]
async fn test_command_info() {
    let info = CommandInfo::builtin("help", "Show help", vec!["?".to_string()]);
    assert_eq!(info.name, "help");
    assert_eq!(info.command_type, CommandType::Builtin);
}
