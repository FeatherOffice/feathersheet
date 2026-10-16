//! O servidor MCP do FeatherSheet. A lógica das ferramentas fica aqui, sobre um
//! [`hospede::Hospede`]: a pasta aberta por arquivo ([`hospede::Avulso`]) ou o
//! próprio aplicativo, ao vivo (o `feathersheet-gui` implementa o hospedeiro e
//! atende o canal de [`ao_vivo`]).
//!
//! Regra central (ver `feathersheet/CLAUDE.md`, "MCP do FeatherSheet"): nunca
//! despejar a planilha para a IA. Ler tem teto de células; resumo, estatística
//! e consulta são calculados pelo motor, e a IA recebe só o resultado.
pub mod ao_vivo;
pub mod arquivo;
pub mod dados;
pub mod esquemas;
pub mod ferramentas;
pub mod formato;
pub mod hospede;
pub mod macros;
pub mod rpc;
pub mod texto;
