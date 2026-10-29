#!/usr/bin/perl
# Session ownership, independent of a login shell's PATH and inherited chat/URL.
# Bindings require a live CLI process. The owning folder also covers tools run by a shared executor.
use strict;
use warnings;
use JSON::PP;
use File::Path qw(make_path);
use Fcntl qw(O_WRONLY O_CREAT O_EXCL);
use Cwd qw(getcwd abs_path);

sub process_identity {
  my ($pid) = @_;
  return unless $pid =~ /^\d+$/ && $pid > 1;
  if (open(my $f, '<', "/proc/$pid/stat")) {
    my $s = <$f>; close $f;
    $s =~ s/^.*\) //;
    my @v = split /\s+/, $s;
    return ($v[1], $v[19]); # parent and start ticks, safe against pid reuse
  }
  # macOS has no /proc. ps reads no environment or command arguments here.
  open(my $p, '-|', 'ps', '-p', $pid, '-o', 'ppid=', '-o', 'lstart=') or return;
  my $s = <$p> // ''; close $p;
  return ($1, $2) if $s =~ /^\s*(\d+)\s+(.+?)\s*$/;
  return;
}

sub route_dir { return "$ENV{HOME}/.shepherdr/helper/routes"; }

sub read_route {
  my ($file, $pid) = @_;
  open(my $f, '<', $file) or return;
  my $r = eval { local $/; JSON::PP->new->decode(<$f>) }; close $f;
  my (undef, $stamp) = process_identity($pid);
  return $r if $r && defined($stamp) && "$r->{stamp}" eq "$stamp";
  return;
}

sub agent_executable {
  my ($exe, $entry) = @_;
  return 1 if ($exe // '') =~ m{(?:^|/)(?:codex|claude|pi)$};
  return 0 unless ($exe // '') =~ m{(?:^|/)(?:node|bun)$};
  return 1 if ($entry // '') =~ m{(?:^|/)(?:codex|claude|pi)(?:\.js)?$};
  return ($entry // '') =~ m{/(?:pi-coding-agent|claude-code|\@openai/codex)/.*\.(?:m?js|cjs)$};
}

sub cli_environment {
  my ($pid) = @_;
  if (open(my $f, '<', "/proc/$pid/cmdline")) {
    local $/; my $cmd = <$f>; close $f;
    my ($exe, @args) = split /\0/, $cmd;
    return unless agent_executable($exe, $args[0]);
    open(my $e, '<', "/proc/$pid/environ") or return;
    my $env = <$e>; close $e;
    my %v = map { split /=/, $_, 2 } grep { /=/ } split /\0/, $env;
    return { url => $v{SHEPHERDR_URL} // '', chat => $v{SHEPHERDR_CHAT} // '', shared => scalar(grep { $_ eq 'app-server' } @args) };
  }
  open(my $p, '-|', 'ps', '-p', $pid, '-o', 'comm=') or return;
  my $exe = <$p> // ''; close $p; chomp $exe;
  open(my $a, '-|', 'ps', '-p', $pid, '-o', 'command=') or return;
  my $args = <$a> // ''; close $a;
  my ($first, $entry) = split /\s+/, $args =~ s/^\s+//r;
  return unless agent_executable($exe, $entry) || agent_executable($first, $entry);
  open(my $e, '-|', 'ps', 'eww', '-p', $pid, '-o', 'command=') or return;
  local $/; my $env = <$e> // ''; close $e;
  return { shared => ($env =~ /\bapp-server\b/ ? 1 : 0), url => ($env =~ /(?:^|\s)SHEPHERDR_URL=(\S*)/ ? $1 : ''), chat => ($env =~ /(?:^|\s)SHEPHERDR_CHAT=(\S*)/ ? $1 : '') };
}

sub session_route {
  my $pid = getppid();
  my (%seen, $ancestor, $executor);
  my $owning_ancestry = 1;
  for (1 .. 64) {
    last if $seen{$pid}++;
    my ($parent, $stamp) = process_identity($pid);
    last unless defined $stamp;
    # Only the long-lived CLI/executor supplies a stale baseline. A caller's shell/env is explicit.
    my $cli = cli_environment($pid);
    $executor //= $cli;
    # Nothing above a shared daemon belongs to the calling task, including the CLI that started it.
    $owning_ancestry = 0 if $cli && $cli->{shared};
    $ancestor //= read_route(route_dir() . "/$pid.json", $pid) if $owning_ancestry;
    $pid = $parent;
  }
  my (@best, $explicit);
  my $chat = $ENV{SHEPHERDR_CHAT} // '';
  if (length($chat) && opendir(my $d, route_dir())) {
    my $cwd = getcwd();
    while (my $name = readdir($d)) {
      next unless $name =~ /^(\d+)\.json$/;
      my $r = read_route(route_dir() . "/$name", $1) or next;
      # A launch identity explicitly supplied to a shared tool also works outside its folder.
      $explicit = $r if $executor && $chat ne $executor->{chat} && $r->{chat} eq $chat;
      my $at = $r->{cwd} // ''; next unless length($at);
      next unless $cwd eq $at || index($cwd, "$at/") == 0;
      @best = () if @best && length($at) > length($best[0]->{cwd});
      push @best, $r if !@best || length($at) == length($best[0]->{cwd});
    }
    closedir($d);
  }
  # Credentials never follow caller-selected chat/cwd. Only process ancestry or a credential
  # reference provisioned in the launch environment can supply proof to the command.
  my $credential = $ancestor;
  if (!$credential && (my $file = $ENV{SHEPHERDR_SECRET_FILE})) {
    if (open(my $f, '<', $file)) {
      my $v = eval { local $/; JSON::PP->new->decode(<$f>) }; close $f;
      $credential = read_route($file, $v->{pid}) if $v && $v->{pid};
    }
  }
  my $r = $explicit // $ancestor;
  if (!$r && @best) {
    ($r) = grep { $_->{chat} eq $chat } @best;
    if (!$r) {
      my %owners = map { my $c = $_->{chat}; $c =~ s{/(Agent|Reviewer)$}{}; ($c => 1) } @best;
      die "Several live sessions own this folder. Set SHEPHERDR_CHAT to the identity in your launch prompt.\n" if keys(%owners) > 1;
      ($r) = grep { $_->{chat} !~ m{/Reviewer$} } @best;
      $r //= $best[0];
    }
  }
  if ($r) {
    $r = { %$r }; # do not let a folder match carry its credential
    delete $r->{capability};
    $r->{capability} = $credential->{capability} if $credential && $credential->{chat} eq $r->{chat};
    $r->{toolInheritedUrl} = $executor->{url} if $executor;
  }
  return $r;
}

sub routed_chat {
  my ($r) = @_;
  my $chat = $ENV{SHEPHERDR_CHAT} // '';
  # A child reviewer can deliberately set its identity. An inherited stale identity cannot win.
  my $owner = $r->{chat} // '';
  my $base = $owner; $base =~ s{/(Agent|Reviewer)$}{};
  return $chat if $chat eq "$base/Reviewer";
  return $r->{chat};
}

sub bind_route {
  my ($pid, $json) = @_;
  $json //= do { local $/; <STDIN> };
  my (undef, $stamp) = process_identity($pid);
  die "Agent process $pid is gone\n" unless defined $stamp;
  my $r = JSON::PP->new->decode($json);
  $r->{stamp} = $stamp;
  $r->{pid} = 0 + $pid;
  $r->{cwd} = abs_path($r->{cwd}) // $r->{cwd} if $r->{cwd};
  if (open(my $f, '<', "/proc/$pid/environ")) {
    local $/; my $e = <$f>; close $f;
    my %env = map { split /=/, $_, 2 } grep { /=/ } split /\0/, $e;
    $r->{inheritedUrl} = $env{SHEPHERDR_URL} // '';
    $r->{inheritedChat} = $env{SHEPHERDR_CHAT} // '';
  } else {
    # macOS: read privately, retain only the two non-secret routing values.
    open(my $p, '-|', 'ps', 'eww', '-p', $pid, '-o', 'command=') or die "ps: $!";
    local $/; my $e = <$p> // ''; close $p;
    $r->{inheritedUrl} = $e =~ /(?:^|\s)SHEPHERDR_URL=(\S*)/ ? $1 : '';
    $r->{inheritedChat} = $e =~ /(?:^|\s)SHEPHERDR_CHAT=(\S*)/ ? $1 : '';
  }
  my $d = route_dir();
  die "Unsafe route directory\n" if -l $d;
  make_path($d, { mode => 0700 });
  die "Unsafe route directory owner\n" unless (stat($d))[4] == $<;
  chmod(0700, $d) or die "route directory permissions: $!";
  my $tmp = "$d/$pid.$$.tmp";
  sysopen(my $f, $tmp, O_WRONLY | O_CREAT | O_EXCL, 0600) or die "route file creation failed: $!";
  chmod(0600, $tmp) or die "route permissions: $!";
  print $f JSON::PP->new->canonical->encode($r); close $f;
  rename($tmp, "$d/$pid.json") or die "route: $!";
  if (my $file = $r->{credentialFile}) {
    my $base = "$ENV{HOME}/.shepherdr/helper/credentials";
    die "Invalid credential path\n" unless $file =~ m{^\Q$base\E/[a-f0-9-]{36}$};
    die "Unsafe credential directory\n" if -l $base;
    make_path($base, { mode => 0700 });
    die "Unsafe credential owner\n" unless (stat($base))[4] == $<;
    chmod(0700, $base) or die "credential permissions: $!";
    my $tmp = "$file.$$.tmp";
    sysopen(my $f, $tmp, O_WRONLY | O_CREAT | O_EXCL, 0600) or die "credential creation failed: $!";
    print $f JSON::PP->new->canonical->encode($r); close $f or die "credential write failed: $!";
    rename($tmp, $file) or die "credential rename failed: $!";
  }
}

unless (caller) {
  if (($ARGV[0] // '') eq '--bind') { shift @ARGV; bind_route(@ARGV); exit 0; }
  my $local = shift @ARGV;
  delete $ENV{SHEPHERDR_SECRET_CAPABILITY};
  # SHEPHERDR_URL changed by the caller remains explicit; --url also targets even the inherited URL deliberately.
  if (($ARGV[0] // '') eq '--url') {
    shift @ARGV; $ENV{SHEPHERDR_URL} = shift @ARGV // die "--url needs a URL\n";
  } else {
    my $r = session_route();
    if ($r && (!length($ENV{SHEPHERDR_URL} // '') || ($ENV{SHEPHERDR_URL} // '') eq ($r->{inheritedUrl} // '') || ($ENV{SHEPHERDR_URL} // '') eq ($r->{url} // '') || (defined($r->{toolInheritedUrl}) && ($ENV{SHEPHERDR_URL} // '') eq $r->{toolInheritedUrl}))) {
      $ENV{SHEPHERDR_CHAT} = routed_chat($r);
      $ENV{SHEPHERDR_SECRET_CAPABILITY} = $r->{capability} // '';
      if ($r->{helper}) { exec($r->{helper}, @ARGV) or die "helper: $!"; }
      $ENV{SHEPHERDR_URL} = $r->{url};
    }
  }
  exec('bun', $local, @ARGV) or die "bun: $!";
}
1;
