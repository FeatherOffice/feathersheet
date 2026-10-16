# FeatherSheet — divisórias da grade (referência: Google Sheets)

Implementação em Rust/Iced, no Canvas de `feathersheet/crates/feathersheet-gui/src/grade.rs`.

## Ajuste visual

Na planilha nativa, os cabeçalhos ficam visualmente separados do corpo:
- **Vertical fixa**: na fronteira entre os números das linhas e as células (x = `CABECALHO_LINHA`).
- **Horizontal fixa**: na fronteira entre as letras das colunas e as células (y = `CABECALHO_COLUNA`).
- **Separação individual dos cabeçalhos**: linhas entre números consecutivos e colunas consecutivas, respeitando larguras e alturas configuradas.
- A borda fixa é discretamente mais escura que a separação individual, em temas claro e escuro.
- As divisórias dos cabeçalhos independem da opção **Exibir → Linhas de grade**; os separadores entre células continuam controlados pela opção.
- Renderização limitada ao viewport; sem percorrer a planilha inteira. Cabeçalhos com largura ou altura zero não criam divisória falsa.
- O zoom é compensado para preservar espessura aproximada em pixels físicos. A pintura é exclusivamente visual, não modifica o Workbook ou o arquivo FSHZ.

## Como testar

Execute no diretório raiz do repositório:

```bash
git switch main
git pull --ff-only origin main

cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml divisor_do_cabecalho -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml divisorias_sao_independentes -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Teste visual:** abra o FeatherSheet, confira a borda contínua sob o cabeçalho A/B/C e à direita da numeração 1/2/3; confira linhas finas separando letras e números. Selecione A12, role as linhas e colunas, mude zoom para 50% e 200%, alterne tema claro/escuro e desmarque **Exibir → Linhas de grade**: as duas bordas fixas e a separação entre cabeçalhos devem continuar aparecendo.

**Validação:** foram criados dois testes unitários. Compilação e aceitação visual precisam ser realizadas em máquina com Cargo/rustc, indisponíveis no ambiente deste agente.
