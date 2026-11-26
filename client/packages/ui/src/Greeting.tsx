// 土台の確認に使う最小の部品。props で文言を受け、見出しとして出す。
export interface GreetingProps {
  title: string;
  message: string;
}

export function Greeting({ title, message }: GreetingProps) {
  return (
    <section aria-labelledby="greeting-title">
      <h1 id="greeting-title">{title}</h1>
      <p>{message}</p>
    </section>
  );
}
