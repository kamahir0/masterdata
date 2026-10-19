import type { ReactNode } from "react";
import { Button, Tooltip } from "antd";
import { DeleteOutlined } from "@ant-design/icons";

export function DefinitionItem({
  identity,
  name,
  removeLabel,
  onRemove,
  children,
}: {
  identity: string;
  name: ReactNode;
  removeLabel: string;
  onRemove: () => void;
  children: ReactNode;
}) {
  return (
    <section className="definition-item" data-motion-key={identity}>
      <div className="definition-item-header">
        <div className="definition-item-name">{name}</div>
        <Tooltip title={`${removeLabel}（項目全体）`}>
          <Button
            type="text"
            danger
            icon={<DeleteOutlined />}
            aria-label={removeLabel}
            onClick={onRemove}
          />
        </Tooltip>
      </div>
      <div className="definition-item-body">{children}</div>
    </section>
  );
}
