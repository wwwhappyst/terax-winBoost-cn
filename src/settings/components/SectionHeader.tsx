type Props = {
  title: string;
  description?: string;
};

export function SectionHeader({ title, description }: Props) {
  return (
    <div className="flex flex-col gap-1">
      <h1 className="text-[18px] font-semibold tracking-tight">{t(title)}</h1>
      {description ? (
        <p className="text-[12px] text-muted-foreground">{t(description)}</p>
      ) : null}
    </div>
  );
}
import { t } from "@/modules/i18n";
