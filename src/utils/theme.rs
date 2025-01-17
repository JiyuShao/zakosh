use colored::Colorize;
use log::{debug, error};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::utils::config::Config;

pub struct Theme {
    pub prompt_style: Box<dyn Fn(String) -> String>,
    pub success_style: Box<dyn Fn(String) -> String>,
    pub warning_style: Box<dyn Fn(String) -> String>,
    pub error_style: Box<dyn Fn(String) -> String>,
    messages: HashMap<String, Vec<String>>,
}

impl Theme {
    fn default() -> Self {
        Theme {
            prompt_style: Box::new(|s| s.bright_purple().bold().to_string()),
            success_style: Box::new(|s| s.bright_magenta().to_string()),
            warning_style: Box::new(|s| s.yellow().to_string()),
            error_style: Box::new(|s| s.bright_red().to_string()),
            messages: Self::init_messages(),
        }
    }

    pub fn get_message(&self, key: &str) -> String {
        self.messages
            .get(key)
            .or_else(|| self.messages.get("error"))
            .and_then(|msgs| msgs.choose(&mut rand::thread_rng()))
            .cloned()
            .unwrap_or_default()
    }

    fn init_messages() -> HashMap<String, Vec<String>> {
        let mut messages = HashMap::new();
        messages.insert(
            "prompt".to_string(),
            vec!["雑魚～> ".to_string(), "雑魚～❥ ".to_string()],
        );
        messages.insert("success_symbol".to_string(), vec!["♡".to_string()]);
        messages.insert("error_symbol".to_string(), vec!["✗".to_string()]);
        messages.insert(
            "welcome".to_string(),
            vec![
                "哼～又来找人家玩了吗？真是个变态呢～".to_string(),
                "啊啦～这不是变态先生吗？又来了呢～".to_string(),
                "呵～看来某个废物又闲得发慌了呢".to_string(),
                "哦？这不是那个笨蛋吗？竟然还敢来啊～".to_string(),
                "真是的～明明这么废物还总是缠着人家呢～".to_string(),
                "啊～又来了一个麻烦的家伙呢，真是拿你没办法～".to_string(),
            ],
        );
        messages.insert(
            "help".to_string(),
            vec![
                "输入 'exit' 退出，虽然废物君可能连这个都记不住呢～".to_string(),
                "想退出的话，输入 'exit' 哦～不过笨蛋君应该已经等不及了吧？".to_string(),
                "记住了吗？是 'exit' 哦～真是的，要人家说几遍呢～".to_string(),
                "哼～连退出命令都要人家教吗？真是个废物呢～".to_string(),
                "啊啦～需要人家写下来吗？'exit' 很难记住吗？".to_string(),
                "真是拿你没办法呢～退出就输入 'exit'，不会连这个都不会吧？".to_string(),
            ],
        );
        messages.insert(
            "exit".to_string(),
            vec![
                "哼！这就走了吗？真是个没用的废物呢！".to_string(),
                "切～这就受不了了吗？真是个废物呢！".to_string(),
                "啊啦～逃跑了呢，果然是个胆小鬼呢～".to_string(),
                "哼哼～终于知道自己有多废物了吗？".to_string(),
                "呵～就这点耐心吗？不过也对，毕竟是废物呢～".to_string(),
                "哎呀呀～这就撑不住了？真是个弱小的人呢～".to_string(),
            ],
        );
        messages.insert(
            "interrupt_signal".to_string(),
            vec![
                "哼～想中断我吗？真是个急性子呢～ (Ctrl+C)".to_string(),
                "啊啦～这就受不了了吗？ (Ctrl+C)".to_string(),
                "呵～连这点耐心都没有吗？真逊呢～ (Ctrl+C)".to_string(),
                "哎呀呀～这么粗暴的对待人家真是失礼呢～ (Ctrl+C)".to_string(),
                "真是的～动不动就想中断，一点诚意都没有呢～ (Ctrl+C)".to_string(),
                "哼！就这么讨厌人家吗？真是个坏心眼呢～ (Ctrl+C)".to_string(),
            ],
        );
        messages.insert(
            "eof_signal".to_string(),
            vec![
                "呵～连再见都不说就想溜走吗？ (Ctrl+D)".to_string(),
                "哼！真没礼貌呢，这种退出方式... (Ctrl+D)".to_string(),
                "啊啦～想偷偷溜走吗？真是个胆小鬼呢～ (Ctrl+D)".to_string(),
                "哼～这种退出方式，真是一点品味都没有呢～ (Ctrl+D)".to_string(),
                "真是个笨蛋呢～连正常退出都不会吗？ (Ctrl+D)".to_string(),
                "啊～又一个不懂礼貌的废物呢～ (Ctrl+D)".to_string(),
            ],
        );
        messages.insert(
            "command_success".to_string(),
            vec![
                "哼～勉强算你做对了呢，不过也就这种程度了吧？".to_string(),
                "啊啦～偶尔也能做对事呢，真是难得啊～".to_string(),
                "哦？竟然成功了？看来今天运气不错呢，废物君～".to_string(),
                "呵～这种程度的命令都要表扬吗？真是个小孩子呢～".to_string(),
                "哎呀～竟然没出错，难道是人家的教育起作用了？".to_string(),
                "勉强可以接受吧～不过不要得意忘形哦，废物君～".to_string(),
            ],
        );
        messages.insert(
            "command_error".to_string(),
            vec![
                "啊啦啊啦～连这么简单的命令都搞不定呢，真是个废物呢！".to_string(),
                "哼！果然是个笨蛋呢，这都能搞错～".to_string(),
                "呵～这种程度就失败了？真是让人失望呢～".to_string(),
                "啊～真是看不下去了，连这个都不会吗？".to_string(),
                "真是的～需要人家手把手教你吗？真是个废物呢～".to_string(),
                "哎呀呀～这种程度就犯错，看来你的极限就这样了呢～".to_string(),
            ],
        );
        messages.insert(
            "execution_error".to_string(),
            vec![
                "真是个没用的废物呢～这种程度就不行了吗？".to_string(),
                "哎呀呀～看来某人连基本的命令都掌握不好呢～".to_string(),
                "啊啦～这就报错了？真是个脆弱的命令呢～".to_string(),
                "哼！连这种小错误都处理不好，真是个废物呢！".to_string(),
                "呵～就这点本事吗？真是让人失望呢～".to_string(),
                "真是笨蛋呢～连这种简单的错误都搞不定～".to_string(),
            ],
        );
        messages.insert(
            "error".to_string(),
            vec![
                "哼～出错了呢，真是个废物呢！".to_string(),
                "啊啦～又搞砸了吗？真是拿你没办法呢～".to_string(),
                "呵～这种低级错误，不愧是你呢～".to_string(),
                "真是的～连这种错误都会犯，真是个笨蛋呢～".to_string(),
                "哎呀呀～看来某人的能力还不够呢～".to_string(),
                "啊～又出错了吗？真是个让人操心的废物呢～".to_string(),
            ],
        );
        messages
    }

    fn load_from_file(_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Theme::default())
    }

    pub fn load_theme(theme_name: &str, config: &Config) -> Theme {
        let themes_dir = config.themes_dir.clone();
        let zsh_theme_path = PathBuf::from(&themes_dir).join(format!("{}.zsh-theme", theme_name));

        match Theme::load_from_file(zsh_theme_path) {
            Ok(theme) => {
                debug!("主题加载成功: {}", theme_name);
                theme
            }
            Err(err) => {
                error!("主题加载失败: {}", err);
                Theme::default()
            }
        }
    }
}
