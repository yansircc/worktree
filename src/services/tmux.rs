use crate::error::Result;
use crate::services::command::CommandRunner;

/// 检查 session 是否存在
pub fn session_exists(session: &str) -> bool {
    CommandRunner::tmux().success(&["has-session", "-t", session])
}

/// 创建独立 session 并执行命令（用于任务启动）
/// 每个任务/scratch 环境都有自己的独立 session
pub fn create_session_with_command(session: &str, cwd: &str, command: &str) -> Result<()> {
    // 创建 session（-d 后台创建，-c 设置工作目录）
    CommandRunner::tmux().run(&["new-session", "-d", "-s", session, "-c", cwd])?;

    // 用 send-keys 发送命令，这样 shell 别名也能生效
    CommandRunner::tmux().run(&["send-keys", "-t", session, "-l", command])?;
    CommandRunner::tmux().run(&["send-keys", "-t", session, "Enter"])
}

/// 关闭整个 session
pub fn kill_session(session: &str) -> Result<()> {
    CommandRunner::tmux().run(&["kill-session", "-t", session])
}

/// 如果 session 存在则关闭，返回是否执行了关闭操作
pub fn kill_session_if_exists(session: &str) -> Result<bool> {
    if session_exists(session) {
        kill_session(session)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
