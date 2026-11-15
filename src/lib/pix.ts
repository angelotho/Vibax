/** Chave Pix aleatória (EVP) para doações ao projeto. */
export const PIX_KEY = "8869da3c-6639-4ee3-83b4-cb9d6fcec796"

// Nome e cidade são só informativos no QR estático: o app do banco sempre mostra o titular real.
const MERCHANT_NAME = "VIBAX"
const MERCHANT_CITY = "BRASIL"

const field = (id: string, value: string) => id + value.length.toString().padStart(2, "0") + value

/** CRC16-CCITT (poly 0x1021, init 0xFFFF), exigido pelo padrão EMV do BR Code. */
export function crc16(payload: string): string {
  let crc = 0xffff
  for (let i = 0; i < payload.length; i++) {
    crc ^= payload.charCodeAt(i) << 8
    for (let b = 0; b < 8; b++) crc = crc & 0x8000 ? ((crc << 1) ^ 0x1021) & 0xffff : (crc << 1) & 0xffff
  }
  return crc.toString(16).toUpperCase().padStart(4, "0")
}

/** Monta o "Pix copia e cola" estático (sem valor definido: quem doa escolhe). */
export function pixPayload(key = PIX_KEY, name = MERCHANT_NAME, city = MERCHANT_CITY): string {
  const body =
    field("00", "01") +
    field("26", field("00", "br.gov.bcb.pix") + field("01", key)) +
    field("52", "0000") +
    field("53", "986") +
    field("58", "BR") +
    field("59", name.slice(0, 25)) +
    field("60", city.slice(0, 15)) +
    field("62", field("05", "***")) +
    "6304"
  return body + crc16(body)
}
