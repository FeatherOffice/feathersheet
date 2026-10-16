# Migrar `macros/api-formato.ts` para Rust

**Data:** 12/10/2026. Alterações diretas na `main`, commits `[skip ci]`.
**Sem executar** `cargo`, GitHub Actions ou CI nesta sessão.

## Comandos já operacionais no host Boa/Rust

Fonte: `feathersheet/src/macros/api-formato.ts`.
Implementação: `feathersheet/crates/feathersheet-macros/src/operacoes_formato.rs`
e `src/bordas.rs`, conectadas por `src/lib.rs`.

| Método JavaScript | Rust / regras |
| --- | --- |
| `setFontFamily(area,nome)` | Família de fonte, sem nome vazio |
| `setFontSize(area,px)` | Tamanho em pixels (0,400] |
| `setBold(area,v=true)` | Negrito |
| `setItalic(area,v=true)` | Itálico |
| `setUnderline(area,v="single")` | Single/double = sublinhado, none=false |
| `setStrikethrough(area,v=true)` | Tachado |
| `setFontColor(area,cor)` | `#RGB`, `#RRGGBB` e BGR VBA |
| `setBackground(area,cor)` | Cor de fundo |
| `setAlignHorizontal(area,v)` | left, center, right ou general |
| `setAlignVertical(area,v)` | top, middle, bottom |
| `setWrapText(area,v=true)` | Quebra de linha |
| `setTextRotation(area,graus)` | Arredondado/clamp [-90,90] |
| `setHyperlink(area,url,texto?)` | Link aplicado ao intervalo; texto opcional somente na primeira célula |
| `setBorder(area,opcoes)` | top,bottom,left,right,insideH,insideV,all,none,color |
| `setOutline(area,opcoes?)` | Contorno externo, cor e largura suportadas |

Esses comandos podem ser chamados como `sheet.setBold("A1:B5")`
ou `sheet.range("A1:B5").setBold()`. Comandos de intervalo devolvem
o próprio objeto quando apropriado para permitir encadeamento.

### Exemplo

```javascript
sheet.setBold("A1:D1");
sheet.setBackground("A1:D1", "#2563eb");
sheet.setFontColor("A1:D1", "#fff");
sheet.setAlignHorizontal("A1:D1", "center");
sheet.setOutline("A1:D10", { color: "#888" });
sheet.range("D3").setHyperlink("https://example.org", "Abrir");
```

### Integridade

- Valida argumentos (cor, tamanho, alinhamento, espessura, diagonal)
  antes de editar. Bordas aceitam `solid` com largura `1` e rejeitam
  diagonal; não há promessa de estilos ainda inexistentes no produto.
- Aplica `Sheet::snapshot` / `Sheet::apply(record=true)` em vez de
  criar histórico por célula. Respeita a transação única da macro e o
  desfazer quando o script lançar erro.
- Um estilo herdado de aba/linha/coluna **não é convertido em estilo
  direto** por uma chamada de `setBold` ou `setItalic`. Uma borda
  recompõe as bordas anteriores da célula, sem apagar outras opções.
- Teto de **500 mil células** para operações de formato em geral
  (limite do host existente) e **20 mil células** para bordas,
  como no TS.
- `setHyperlink` nunca altera o texto de outras células do intervalo.

### Pendências

- `setShowGridlines(v)` é uma opção visual da janela (não estilo do
  documento). Precisa de integração ao estado Iced e à preferência de
  exibição; **não foi simulada com metadados**.
- Áreas de linha/coluna completa ainda estão sujeitas ao limite do
  host; otimizar persistência da formatação de coluna inteira exige
  outro caminho por metadados (não iterar milhões de células).
- Paridade do formato avançado de bordas (largura diferente de 1,
  pontilhados e diagonais) não existe no próprio modelo do core.
- Testes escritos em `operacoes_formato.rs`, `bordas.rs` e
  `testes_formato.rs`; **não executados**. Sugestão local:
  `cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2`.

### Descarte do TypeScript

`feathersheet/src/macros/api-formato.ts` ainda é importado por
`feathersheet/src/macros/runtime.ts`. Não o retirar isoladamente,
pois isso invalidaria a versão legada enquanto seu runtime continuar
presente. Essa migração executa os comandos no Rust sem depender desse
arquivo TypeScript. O próximo descarte seguro depende de retirar/
substituir os **imports ativos do runtime legado**, não de apagá-los
antecipadamente.
