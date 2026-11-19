export function health(_req: unknown, res: { status: (n:number)=> { json:(b:unknown)=>void } }) {
  return res.status(200).json({ ok: true });
}
