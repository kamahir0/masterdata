import React from "react";
import { Button, Result } from "antd";
export class EditorBoundary extends React.Component<
  { revision: number; onSource: () => void; children: React.ReactNode },
  { error: string | null }
> {
  state = { error: null as string | null };
  static getDerivedStateFromError(error: Error) {
    return { error: error.message };
  }
  componentDidUpdate(previous: Readonly<{ revision: number }>) {
    if (previous.revision !== this.props.revision && this.state.error)
      this.setState({ error: null });
  }
  render() {
    if (this.state.error)
      return (
        <Result
          status="warning"
          title="この形式を通常の編集画面で表示できません"
          subTitle="入力は保持しています。ソースから修正するか、元に戻す操作を使えます。"
          extra={
            <Button type="primary" onClick={this.props.onSource}>
              ソースを開いて修正
            </Button>
          }
        />
      );
    return this.props.children;
  }
}
