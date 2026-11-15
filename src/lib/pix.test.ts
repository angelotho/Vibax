import { describe, expect, it } from "vitest"

import { crc16, pixPayload } from "./pix"

describe("pix", () => {
  it("calcula o CRC16-CCITT padrão", () => {
    expect(crc16("123456789")).toBe("29B1")
  })

  it("monta o payload com a chave e CRC válido", () => {
    const p = pixPayload("abc", "NOME", "CIDADE")
    expect(p).toContain("0014br.gov.bcb.pix0103abc")
    expect(p.slice(-4)).toBe(crc16(p.slice(0, -4)))
    expect(p.startsWith("000201")).toBe(true)
  })
})
