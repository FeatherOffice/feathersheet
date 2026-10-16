# Renomear aba — interface nativa

Segue o prompt do TypeScript (`src/ui/app/estrutura.ts` e `src/ui/caixas/dialog.ts`): duplo clique na aba ou botão direito → Renomear abre uma caixa centralizada de 420 px, com fundo escurecido, nome atual selecionado e botões Cancelar/Salvar. Enter salva; Esc ou clique fora cancela. O campo antigo e os botões permanentes Renomear/Excluir foram removidos da barra; excluir continua no menu de contexto.

Erros de nome vazio, `!` ou duplicação sem diferença de maiúsculas aparecem no diálogo, sem fechar ou modificar a aba. O motor existente continua responsável pela renomeação e atualização de referências das fórmulas. O alvo é identificado pelo ID da aba, preservando a intenção mesmo se a ordem mudar.

## Validação pelo mantenedor

Os dois testes de renomeação passaram na suíte da GUI em 08/10/2026, após autorização do mantenedor para executar Cargo. Cobrem abertura por duplo clique, cancelamento, nomes inválidos/repetidos e salvamento.

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml renomear_aba -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```
