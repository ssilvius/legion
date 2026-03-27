import { useCallback, useRef, useState, type KeyboardEvent } from "react";
import { Container } from "@/src/components/ui/container";
import { Textarea } from "@/src/components/ui/textarea";
import { Button } from "@/src/components/ui/button";
import { useLegion } from "@/src/services";

export function BroadcastBar() {
  const legion = useLegion();
  const [sending, setSending] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSend = useCallback(() => {
    const text = textareaRef.current?.value.trim();
    if (!text) return;

    setSending(true);
    legion
      .post("meatbag", text)
      .then(() => {
        if (textareaRef.current) {
          textareaRef.current.value = "";
        }
      })
      .catch(console.error)
      .finally(() => setSending(false));
  }, [legion]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  return (
    <Container as="div" gap="2">
      <Textarea
        ref={textareaRef}
        placeholder="@agent message... or just post to the bullpen"
        rows={2}
        onKeyDown={handleKeyDown}
      />
      <Button size="sm" onClick={handleSend} disabled={sending}>
        {sending ? "Sending..." : "Send"}
      </Button>
    </Container>
  );
}
