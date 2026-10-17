//! `feathersheet-mcp`: lê uma mensagem JSON por linha no stdin e responde no
//! stdout (a lógica está em [`feathersheet_mcp::rodar_stdio`], que o
//! aplicativo também usa com `feathersheet --mcp`).
fn main() {
    feathersheet_mcp::rodar_stdio();
}
