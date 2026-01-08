import { Provider } from "@/provider";
import { Auth } from "objectiveai";
import { ReactElement, useEffect, useMemo, useRef, useState } from "react";
import cn from "classnames";
import { openAi, SelectorDropdown } from "@/components/Common";
import { ConfirmPopup } from "../../Popup";
import DatePicker from "react-datepicker";
import "react-datepicker/dist/react-datepicker.css";

export function Header({
  sort,
  onSortChanged,
  onAddKey,
  session,
  loading,
  now,
  className,
}: {
  sort: "created" | "cost";
  onSortChanged: (sort: "created" | "cost") => void;
  onAddKey: (key: Auth.ApiKey) => void;
  session: Provider.TokenSession;
  loading?: boolean;
  now: number;
  className?: string;
}): ReactElement {
  const [creating, setCreating] = useState(false);

  const sortItemName = (sort: "created" | "cost"): string => {
    switch (sort) {
      case "created":
        return "Newest";
      case "cost":
        return "Most Dollars Spent";
    }
  };

  return (
    <>
      <div
        className={cn(
          "flex",
          "justify-between",
          "gap-1",
          "w-[calc(var(--spacing)*192)]",
          "max-w-[calc(100%-var(--spacing)*4)]",
          "mx-auto",
          "py-1",
          "border-b",
          "border-muted-secondary",
          className
        )}
      >
        <button
          className={cn(
            creating ? "bg-highlight-secondary" : "hover:bg-highlight-muted",
            loading ? "cursor-wait" : "cursor-pointer",
            "transition-colors",
            "duration-200",
            "px-2",
            "rounded-lg"
          )}
          onClick={() => setCreating(!creating)}
          disabled={loading}
        >
          Create API Key
        </button>
        <SelectorDropdown<"created" | "cost">
          items={["created", "cost"]}
          selected={sort}
          onChange={onSortChanged}
          itemName={sortItemName}
          placement="bottom-end"
          expanderLocation="left"
          popupClassName={cn("whitespace-nowrap")}
        >
          {sortItemName}
        </SelectorDropdown>
      </div>
      {creating && (
        <CreatePopup
          session={session}
          onSuccess={(key) => {
            onAddKey(key);
            setCreating(false);
          }}
          onCancel={() => setCreating(false)}
          now={now}
        />
      )}
    </>
  );
}

function CreatePopup({
  onSuccess,
  onCancel,
  session,
  now,
}: {
  onSuccess: (key: Auth.ApiKey) => void;
  onCancel: () => void;
  session: Provider.TokenSession;
  now: number;
}): ReactElement {
  const tomorrowMidnight = useMemo(() => {
    const date = new Date(now);
    date.setHours(24, 0, 0, 0);
    return date;
  }, [now]);
  const [name, setName] = useState("");
  const nameTrimmed = name.trim();
  const [description, setDescription] = useState("");
  const [expires, setExpires] = useState<Date>();
  const [loading, setLoading] = useState(false);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  const handleSave = () => {
    if (nameTrimmed === "") return;
    setLoading(true);
    let descriptionTrimmed: string | undefined = description.replace(
      /^[\n\s]+|[\n\s]+$/g,
      ""
    );
    if (descriptionTrimmed === "") descriptionTrimmed = undefined;
    (async () => {
      const openai = await openAi(session);
      const key = await Auth.ApiKey.create(
        openai,
        nameTrimmed,
        expires,
        descriptionTrimmed === "" ? undefined : descriptionTrimmed
      );
      setLoading(false);
      onSuccess(key);
    })();
  };

  const resizeTextArea = () => {
    if (textAreaRef.current) {
      textAreaRef.current.style.height = "auto"; // Reset height
      textAreaRef.current.style.height = `${textAreaRef.current.scrollHeight}px`; // Set to scrollHeight
    }
  };

  useEffect(resizeTextArea, [description, textAreaRef]);

  return (
    <ConfirmPopup
      onConfirm={handleSave}
      onCancel={onCancel}
      canConfirm={nameTrimmed !== ""}
      title={"Create API Key"}
      body={
        <div
          className={cn(
            "w-[calc(var(--spacing)*192)]",
            "max-w-[calc(100dvw-var(--spacing)*10)]",
            "space-y-2"
          )}
        >
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="name"
            className={cn(
              "w-full",
              "p-2",
              "border",
              "border-muted-secondary",
              "rounded-lg",
              "outline-none",
              "placeholder-muted-primary"
            )}
            disabled={loading}
          />
          <DatePicker
            selected={expires}
            dateFormat="Pp"
            onChange={(date) => setExpires(date ?? undefined)}
            placeholderText="expires (optional)"
            className={cn(
              "w-[calc(var(--spacing)*192)]",
              "max-w-[calc(100dvw-var(--spacing)*10)]",
              "p-2",
              "border",
              "border-muted-secondary",
              "rounded-lg",
              "outline-none",
              "placeholder-muted-primary"
            )}
            minDate={tomorrowMidnight}
            disabled={loading}
            showTimeSelect
          />
          <div
            className={cn(
              "max-h-[50dvh]",
              "overflow-y-auto",
              "p-2",
              "border",
              "border-muted-secondary",
              "rounded-lg"
            )}
          >
            <textarea
              ref={textAreaRef}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="description (optional)"
              className={cn(
                "w-full",
                "resize-none",
                "outline-none",
                "overflow-hidden",
                "min-h-[50px]",
                "placeholder-muted-primary"
              )}
              disabled={loading}
            />
          </div>
        </div>
      }
    />
  );
}
