# FeatherSheet — proteção de memória ao importar XLSX (08/10/2026)

## Sintoma reportado

Ao abrir uma pasta XLSX aparentemente pequena, o monitor do FeatherSheet
mostrou RSS chegando a ~896 MiB, com uma aba e zero células ocupadas.
**Esse painel não confirma, sozinho, que houve OOM**: é necessário medir o
arquivo e a importação, incluindo o pico `VmHWM`. O arquivo XLSX específico
não foi fornecido, portanto a causa exata daquele caso ainda não foi
reproduzida.

O código anterior apresentava três riscos objetivos:

1. `xlsx_read::xml::parse_xml` montava uma árvore de todas as linhas
   `sheetData/row/c` de uma vez, mesmo quando a maioria das células estava
   vazia e o arquivo ZIP era pequeno.
2. Hiperlinks apontando para `A1:XFD1048576` faziam o código percorrer
   a área retangular inteira e produzir strings temporárias para células
   inexistentes.
3. `entry_text` aceitava até 64 MiB **por entrada XML** (que se expandiam
   muito em memória) e retornava string vazia silenciosamente quando uma
   entrada superava o teto, potencialmente deixando a aba sem dados.

## Correções no Rust

| Local | Proteção | Sem alterar |
|---|---|---|
| `feathersheet-io/src/xlsx_read/xml.rs` | `parse_xml_linhas` entrega cada linha ao callback e libera seus nós XML imediatamente | Parser completo `parse_xml` para outras partes do arquivo |
| `feathersheet-io/src/xlsx_read/sheet.rs` | Armazena somente as células relevantes, limita a 250 mil entradas (dados **e** formatação isolada); processa alturas e metadados restantes depois | Fórmulas compartilhadas, formatos, dimensões, mesclas, regras e links |
| `feathersheet-io/src/xlsx_read/sheet.rs` | Aplica hyperlinks percorrendo apenas as células importadas, não milhões de coordenadas vazias | Links válidos e estilo original |
| `feathersheet-io/src/xlsx_read/mod.rs` | Teto descompactado de 32 MiB por entrada XML, verificação do tamanho declarado **e** leitura limitada; erro explícito para entradas acima do teto | Interface `ler_xlsx_bytes` e ZIP/JSON do importador |
| `feathersheet-ponte/examples/medir_abertura_xlsx.rs` | Relata RSS/HWM mesmo em falha de validação, com erro legível e saída diferente de zero | Não imprime conteúdo nem modifica o XLSX |

**Importante:** arquivos legítimos maiores que 32 MiB de XML por entrada
ou com mais de 250 mil células/estilos importáveis serão rejeitados de
maneira informativa. Esses limites de segurança são provisórios, até
existir importação em lotes diretamente no armazenamento colunar, sem
passar pelo JSON do legado. A importação da GUI já executa fora da
thread principal do Iced e mantém o documento anterior em caso de erro.

## COMO TESTAR NO MEU COMPUTADOR

Da raiz do FeatherOffice:

```bash
git status --short
git pull --ff-only origin main

cargo test --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml --lib -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml xlsx_rust_exporta_e_reabre_sem_typescript -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

### Diagnóstico de RSS do arquivo exato

O exemplo `medir_abertura_xlsx` já existia e agora sinaliza falhas sem
`panic`. Não imprime células nem salva dados da planilha:

```bash
cargo run --release \
  --manifest-path feathersheet/crates/feathersheet-ponte/Cargo.toml \
  --example medir_abertura_xlsx \
  -- "/caminho/para/arquivo.xlsx" 2>&1 | tee /tmp/feathersheet-xlsx-memoria.log
```

Comparar as linhas `bytes`, `XML e JSON`, `pasta Rust`,
`viewport calculado`, `VmRSS` e `VmHWM`.
Esses valores sao coletados pelo **proprio exemplo** em `/proc/self/status`,
nao pelo processo Cargo que compila o exemplo. Na interface, verificar também se
carregou o número correto de células e se o monitor estabiliza.

### Testes de regressão adicionais

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml \
  --lib hiperlink_da_planilha_inteira_so_altera_celulas_existentes -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml \
  --lib pequenas_planilhas_com_milhares_de_linhas_vazias_nao_geram_celulas -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml \
  --lib zip_pequeno_com_xml_expandido_demais_retorna_erro_claro -j 2
```

O workflow `feathersheet-rust.yml` também executa esses casos.

**Status de validação:** código, testes e workflow publicados na `main`,
porém este ambiente não possui Cargo/Rust e o XLSX que causou o problema
não foi enviado. Não atribuir uma redução de RSS medida sem executar o
teste com o arquivo real.
