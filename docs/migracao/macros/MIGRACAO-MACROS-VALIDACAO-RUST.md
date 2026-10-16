# Migração da validação de dados das macros para Rust

**Data:** 12/10/2026 • Branch `main` • Commits com `[skip ci]`.
Não foram executados GitHub Actions nem compilação/testes Cargo.

## O que realmente faltava

A API `feathersheet/src/macros/api-dados.ts` ainda tinha duas
operações sem equivalente no host de macros Rust:

```javascript
sheet.setDataValidation("A1:A10", {
    type: "list", values: ["Sim", "Não"]
});
sheet.range("B1:B10").setDataValidation({
    type: "number", operator: "between", min: 10, max: 20,
    mode: "warn", errorMessage: "Valor fora do intervalo"
});
sheet.clearDataValidation("A1:A10");
```

### Auditoria prévia para evitar duplicação

**Já existia em Rust:**
- `feathersheet-core/src/operacoes/validation.rs`, com o modelo
  `Validacao` / `Regra` e `validar` para lista, intervalo, número,
  data, texto e checkbox;
- a persistência interoperável `Sheet.va`, em JSON, nos arquivos
  FeatherSheet; não precisa criar outro formato ou armazenar células;
- um histórico transacional com `Sheet::alterar_mesclas` e
  `Sheet::alterar_tabelas`, usado como padrão para metadados.

**Foi implementado somente o que faltava:**
- `Sheet::alterar_validacoes(novo,area)` registra um `Passo::Validacoes`
  no histórico: rollback, Ctrl+Z e Ctrl+Y passam a incluir o campo `va`;
- o adaptador `feathersheet-macros/src/validacoes.rs` converte
  os objetos JSON da API TypeScript para os tipos nativos **já existentes**
  e atualiza a lista de regras, removendo as antigas que interceptam a área;
- `feathersheet-macros/src/lib.rs` conecta os métodos em
  `sheet` e `sheet.range(...)`, agrupados na transação da macro.

**Não foi implementado outro avaliador de validações.**
O módulo de fórmulas e a GUI continuam usando
`feathersheet-core::operacoes::validation::validar`.

## Tipos suportados

| `type` TS | Regra Rust | Detalhes |
| --- | --- | --- |
| `list` | `Lista` | `values` convertidos com semântica `String(...)` do JS |
| `list` | `Intervalo` | `source: "A1:A10"`, na mesma aba |
| `number` | `Numero` | `between`, `greaterThan`, `lessThan`, `equal` |
| `date` | `Data` | `between`, `before`, `after` |
| `text` | `Texto` | `contains`, `startsWith`, `email`, `url` |
| `checkbox` | `Caixa` | `checked`/`unchecked`, padrão `Sim`/`Não` |

O `mode: "warn"` corresponde a `QuandoValidacao::Avisar`; o
padrão é `Recusar`. A mensagem `errorMessage` é persistida.

`clearDataValidation` remove as regras cujo **qualquer intervalo
cruza** a seleção, preservando regras disjuntas. Como é operação de
metadados, pode atingir `A1:A100000000` sem materializar células e
sem limite artificial de 500 mil células.

## Integridade e testes

Argumentos e JSON preexistente são validados **antes de modificar**
o documento. Uma macro que escreva células e depois falhe ao montar
uma validação reverte as duas alterações. Comandos idênticos não
marcam o arquivo como alterado.

Os testes escritos (mas **não executados**) ficam em:
- `feathersheet-core/src/planilha/sheet/edicao.rs`:
  undo/redo/rollback do histórico de validação;
- `feathersheet-macros/src/validacoes.rs`:
  regras, serialização, compatibilidade do avaliador e remoção seletiva;
- `feathersheet-macros/src/testes_validacoes.rs`:
  contrato Boa/JavaScript, 10M linhas e desfazer com falha.

Verificação local sem GitHub Actions:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-core/Cargo.toml validacoes_json -j 2
```

## Limitações e descarte

- A origem de lista (`source`) deve estar na mesma aba e ter no máximo
  **10 mil células**, limite aplicado pelo avaliador do core; listas
  literais limitadas a 10 mil itens. Não reivindicar paridade com listas
  arbitrariamente grandes no antigo TS.
- `setDataValidation` não faz a verificação instantânea dos valores
  que já existiam no intervalo; isso é responsabilidade do motor
  de validação no momento da edição, igual à divisão do código atual.
- `addConditionalFormat`, `addColorScale`, `addDataBar` e
  `clearConditionalFormats` ainda não foram migrados.
- **Não mover** `src/macros/api-dados.ts` para `descarte/`:
  `src/macros/runtime.ts` ainda importa `apiDados`, e há
  funcionalidades desse módulo pendentes.
