# Migração das funções de endereços de macros (TS → Rust)

Data: 12/10/2026. Commits na `main`, com `[skip ci]`; sem GitHub Actions.

## O que foi implementado

A lógica de `feathersheet/src/macros/enderecos.ts` foi portada para
`feathersheet/crates/feathersheet-macros/src/enderecos.rs`:

- `r1c1_para_a1`: converte `R[-1]C[2]`, `R3C4` e `RC` em A1,
  com limites de planilha e preservação de textos entre aspas.
- `deslocar_formula`: copia fórmulas A1 respeitando `$`, nomes de
  abas, texto literal e `#REF!` em referência fora dos limites.
- `mascara_para_estilo`: interpreta máscaras Excel usuais, com
  validação explícita; formatos não reconhecidos retornam erro.
- Conversão BGR do VBA para hex, normalização de `#rgb` e `#rrggbb`,
  coordenadas 1-based, união e interseção de retângulos.

Também foram conectadas ao interpretador Boa (`feathersheet-macros`)
três operações até então existentes somente na API TS:

| API de macro | Operação nativa |
| --- | --- |
| `sheet.setFormula("D1:D2", "=A1+B1")` | Copia a fórmula e desloca referências relativas por célula |
| `sheet.range("C1:C2").setFormulaR1C1("=RC[-2]+RC[-1]")` | Traduz R1C1 → A1 em cada linha e coluna |
| `sheet.range("B1").setNumberFormat("R$ #,##0.00")` | Aplica patch de estilo numérico e casas decimais |

Todas usam o limite de 500 mil células já imposto ao host, **sem
interpretar JavaScript adicional**. Escritas e formatação passam pela
transação da macro, para que o usuário possa desfazer ou para que a
operação reverta alterações quando há erro.

## O que ainda falta

- A API JS completa de `macros/api-valores.ts`, que também tem
  cópia/colagem, localização, preenchimento e estilos herdados.
- Paridade avançada de máscaras monetárias/data de outras localidades;
  o parser Rust atual cobre as máscaras usuais documentadas, não toda
  máscara possível do Excel.
- Validação de Cargo e execução visual (não realizadas aqui).

## Política de descarte

O arquivo `src/macros/enderecos.ts` ainda é importado por
`src/macros/api-valores.ts` e `src/macros/api-formato.ts`.
**Não o apagar/mover isoladamente para `descarte/`.** A migração
nativa está ligada ao interpretador Rust, enquanto as entradas TS
remanescentes devem ser movidas em conjunto após substituição dos
consumidores e atualização dos testes legados.

Testes de regressão foram escritos em Rust para R1C1, A1 relativo,
máscaras numéricas, reversão e undo. Nenhuma execução de GitHub Actions.
