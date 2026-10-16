# Migração da API de aplicações de macro do TypeScript para Rust

Origem: `feathersheet/src/macros/api-aplicacao.ts` (interfaces `app.status` e `app.error`).

- `app.status(message)` envia uma mensagem à ponte Rust, retida no `Resultado.status` da execução; a janela Iced usa esse valor na barra inferior depois da macro terminar com sucesso. A transação é desfeita se o JavaScript falhar e a mensagem não substitui o erro na barra.
- `app.error(code, detail?)` usa `ErrorCode::from_code` do core Rust para validar o código, cria o envelope JSON de erro de célula e recupera `__CellError` dentro do Boa com a rotina `__ftRevive` já existente, mantendo `app.isError`.
- Limites preventivos: status a 1.024 caracteres e detalhe a 2.048 caracteres.
- Corrigida a declaração duplicada do módulo `funcoes_busca` em `feathersheet-macros/src/lib.rs`, que causaria erro de compilação.

Escopo: criação de objetos JS `CellError` e status durante execução; persistência de erros tipados em células pelo `sheet.set` não foi migrada nesta alteração. Macros automáticas/eventos permanecem pendentes de implementação.

Foram adicionados testes Rust para status, erro tipado e rollback, **ainda não executados**. Nenhuma action remota deve ser disparada.

Para testar localmente:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
```
