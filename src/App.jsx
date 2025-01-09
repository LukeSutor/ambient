import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";

const formSchema = z.object({
  prompt: z.string().min(1, {
    message: "Prompt must be at least 1 character.",
  }).max(500, {
    message: "Prompt must be at most 500 characters.",
  }),
});

function App() {
  const [modelDownloaded, setModelDownloaded] = useState(true);
  const navigate = useNavigate();

  const form = useForm({
    resolver: zodResolver(formSchema),
    defaultValues: {
      prompt: "",
    },
  });

  function onSubmit(values) {
    console.log(values);
  }

  useEffect(() => {
    async function checkModelDownload() {
      try {
        const result = await invoke("check_model_download");
        setModelDownloaded(result);
        if (!result) {
          navigate("/download");
        }
      } catch (err) {
        console.error(`[ui] Failed to check if models are downloaded. ${err}`);
      }
    }
    checkModelDownload();
  }, [navigate]);

  if (!modelDownloaded) {
    return null; // Prevent rendering if redirecting
  }

  return (
    <div className="flex flex-col justify-center items-center w-screen h-screen">
      <Card className="w-[500px]">
        <CardHeader>
          <CardTitle>Ask anything</CardTitle>
          <CardDescription>What would you like to achieve?</CardDescription>
        </CardHeader>
        <CardContent>
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="flex flex-col space-y-4">
              <FormField
                control={form.control}
                name="prompt"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Prompt</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder="Ask me to do anything..."
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <Button type="submit">Submit</Button>
            </form>
          </Form>
        </CardContent>
      </Card>
      <Button
        className="absolute bottom-4 right-4"
        onClick={() => navigate("/debug")}
      >
        Debug
      </Button>
    </div>
  );
}

export default App;