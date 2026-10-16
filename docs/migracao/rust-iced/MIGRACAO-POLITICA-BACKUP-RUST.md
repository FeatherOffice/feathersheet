# Migração do backup-policy.ts para Rust (12/10/2026)

## Implementação nativa

| Comportamento TypeScript | Código Rust |
|---|---|
| `MAX_CELULAS_COPIA = 300_000` | `feathersheet-io/src/backup_policy.rs` |
| `copiaValeAPena()` | `copia_vale_a_pena()` |
| `custoInsustentavel()` | `custo_insustentavel()` |
| `intervaloDaCopia()` | `intervalo_da_copia()` |
| Política na frequência real do autosave | `intervalo_autosave()` + `App::MonitorPulso` |

O FeatherSheet em Rust agora mede a duração real de
`da_workbook::pasta_gravar_arquivo` e guarda a medição em
`App::auto_salvar_ultimo_custo_ms`. O pulso periódico calcula a
cadência efetiva sem serializar a planilha para estimar tamanho:
usa a soma de `sheet.len()` de todas as abas.

**Regras:** arquivos até 300 mil células seguem o máximo do intervalo
selecionado e `clamp(custo_ms * 10, 800, 30.000)`.
Arquivos acima de 300 mil células usam intervalo mínimo de 30 s. O
usuário pode definir tempo maior. `Salvar` manual ignora inteiramente
a política. Autosave continua restrito a arquivo FSHZ existente,
fora de edição/transformação em lote e sem diálogo de caminho.

A política evita repetição **frequente** de I/O lento; ela não torna
a serialização assíncrona, não reduz sozinha o pico de memória de uma
gravação e não cria backups de recuperação versionados.

## TypeScript que continua no lugar

`src/io/backup-policy.ts` ainda é importado dinamicamente em
`test/io/fshz.test.ts`, que preserva testes de compatibilidade com o
frontend Web. **Não foi movido para `descarte/`**, porque isso quebraria
esses testes enquanto forem executados. O motor nativo não depende
do arquivo TS.

Migrar ou aposentar o teste/entrada TypeScript antes de remover o
arquivo original. Jamais apagar uma referência ativa apenas para
reduzir o contador de arquivos `.ts`.

## Validação

Novos casos em `feathersheet-io/src/backup_policy.rs` e
`feathersheet-gui/src/app/configuracoes_ui.rs` cobrem limite,
custo de 8 s -> intervalo de 30 s e gravação após o cooldown.

Não foram executados testes Cargo nem GitHub Actions nesta rodada.
Commits usam `[skip ci]`, conforme a orientação do mantenedor.
