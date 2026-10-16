# FeatherSheet — Pintar linha/coluna da célula ativa

Implementado em 08/10/2026 no Canvas Rust/Iced.

## Comportamento

A opção **Exibir → Pintar linha/coluna** fica ativada por padrão.

- Selecionando **D4**, pinta as partes visíveis da linha **4** e da coluna **D**, com interseção mais forte em **D4**.
- Ao navegar para **F9** com mouse, setas, Tab, Enter ou campo de endereço, o destaque acompanha a posição ativa.
- Clicando em **Exibir → Pintar linha/coluna**, o recurso desliga e religa. A última escolha é salva automaticamente e recuperada ao reiniciar o FeatherSheet.
- Respeita rolagem, zoom, temas claro/escuro, larguras/alturas especiais e linhas/colunas ocultas.
- O Canvas percorre apenas as células do viewport — não aloca nem percorre todas as 100 milhões de linhas.

**Somente visual:** não preenche estilos do Workbook, não altera formatação condicional, copiar/colar, impressão ou o conteúdo do arquivo FSHZ. A preferência é **persistente por usuário**: a última escolha (ligada/desligada) permanece na próxima abertura do FeatherSheet, inclusive ao criar/abrir outra planilha. Na primeira execução ou com configurações antigas sem o campo, o padrão é **ligado**.

## Arquitetura

| Camada | Arquivo | Função |
|---|---|---|
| Canvas | `feathersheet/crates/feathersheet-gui/src/grade.rs` | `realce_cruz` decide Nenhum / Eixo / Cruzamento; compõe overlay RGBA por célula visível, depois do fundo e antes de barras, texto e bordas |
| Estado | `feathersheet/crates/feathersheet-gui/src/app.rs` | `Msg::RealceEixos(bool)` atualiza `config.realce_eixos` e salva imediatamente; `App::novo()` lê a preferência local. A seleção atual segue `linha` e `coluna`. |
| Preferência local | `feathersheet/crates/feathersheet-gui/src/configuracao.rs` | `Configuracao.realce_eixos` serializado em `config-iced.json` (com `#[serde(default)]` para arquivos antigos). Não altera `.fshz`. |
| Ribbon | `feathersheet/crates/feathersheet-gui/src/app/faixa_nativa.rs` | Botão **Pintar linha/coluna** na aba Exibir |

O destaque usa azul translúcido no tema claro e verde-água translúcido no escuro. A interseção recebe maior opacidade.

### Como funciona ao reabrir

O botão da aba **Exibir** não configura a pasta de trabalho; é uma preferência da
interface FeatherSheet. Desativar salva `"realce_eixos": false` e ativar salva
`"realce_eixos": true` no arquivo local:

- **Linux:** `$XDG_CONFIG_HOME/feathersheet/config-iced.json` quando a variável estiver definida; caso contrário, `~/.config/feathersheet/config-iced.json`.
- **Windows:** segue a pasta existente `XDG_CONFIG_HOME`/`HOME` quando configurada, ou usa `%APPDATA%\feathersheet\config-iced.json` como alternativa.

**Teste de reinício:** abra o FeatherSheet, vá até **Exibir** e desative
**Pintar linha/coluna**. Feche a janela, abra novamente e selecione outra
célula: nenhuma linha/coluna deve receber realce e o botão deve estar
desativado. Repita ativando, fechando e reabrindo: o realce deve continuar
ligado. Não é necessário salvar o arquivo `.fshz` e a mudança não deve
acionar o aviso de planilha não salva.

## Como testar

```bash
git switch main
git pull --ff-only origin main

cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml cruz_da_celula_d4 -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml cruz_move_com_a_celula -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml realce_da_linha_e_coluna -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml realce_linha_coluna_ativo_e_desativo_sobrevivem_reabertura -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml preferencias_de_planilha_persistem -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Validação visual:** selecione D4, confira coluna D e linha 4. Mova para F9, alterne botão em Exibir, use zoom 50%/200% e temas claro/escuro. Inspecione a formatação persistente após salvar e abrir FSHZ.

**Validação pendente:** este agente não tem Cargo/rustc disponível; os testes foram escritos e precisam ser executados no computador do mantenedor.
